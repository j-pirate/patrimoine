#!/usr/bin/env python3
"""Measure an isolated synthetic vault with the real release application services.

Build first:
  cargo build --release -p patrimoine-infrastructure --example qualify_performance --locked
Run:
  python3 scripts/qualify-performance.py --output .local/performance-full --profile full

This bulk seed measures volume, not the cost of recording 100,000 sequential
commits. It uses the official schema from Vault::open, inserts parameterized
trusted fixture rows, and requires a real validating Vault::open before reads.
No user data or schema migration is modified. No third-party Python library.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import select
import sqlite3
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
STAMP = '2026-11-12T00:00:00.000000Z'
PROFILES = {'pilot': (10, 10, 100, 1000, 100), 'full': (100, 100, 10000, 100000, 10000)}
OPERATIONS = ['open', 'registry', 'treasury', 'documents', 'account', 'financial', 'historical', 'individual', 'fx', 'dashboard', 'backup', 'csv_export', 'csv_inspect', 'backup_roundtrip', 'csv_roundtrip']


def identity(value):
    return str(uuid.uuid5(uuid.NAMESPACE_URL, 'patrimoine-performance-1/' + value))


def money(value):
    return format(Decimal(value).quantize(Decimal('0.01')), 'f')


def insert(connection, table, values):
    connection.execute(f'INSERT INTO {table} VALUES ({",".join("?" for _ in values)})', values)


def record(connection, kind, name, change_set, supersedes=None):
    object_id = identity('object/' + name)
    revision_id = identity('revision/' + name + '/' + change_set)
    if supersedes is None:
        insert(connection, 'objects', (object_id, kind, STAMP))
    insert(connection, 'revisions', (revision_id, object_id, change_set, STAMP, supersedes, 0, 'Fixture synthétique de qualification'))
    insert(connection, 'audit_events', (identity('audit/' + revision_id), change_set, object_id, revision_id, 'performance_fixture_seed', 'Source synthétique générée pour les mesures de volume'))
    return object_id, revision_id


def profile_period(connection, kind, object_id, revision_id, change_set):
    _, period_revision = record(connection, 'profile_period', 'period/' + object_id, change_set)
    insert(connection, 'profile_periods', (period_revision, kind, object_id, revision_id, '2025-01-01', None))


def seed(vault, profile):
    people_n, companies_n, assets_n, movements_n, documents_n = PROFILES[profile]
    generation = (vault / 'CURRENT').read_text().strip()
    generation_path = vault / 'generations' / generation
    database = generation_path / 'patrimoine.sqlite3'
    connection = sqlite3.connect(database)
    connection.execute('PRAGMA foreign_keys=ON')
    connection.execute('PRAGMA trusted_schema=OFF')
    change_set = identity('change/initial')
    correction = identity('change/correction')
    started = time.monotonic()
    with connection:
        insert(connection, 'change_sets', (change_set, 1, STAMP, 'Lot initial synthétique de qualification de volume', identity('command/initial')))
        people, companies = [], []
        for kind, count, values in [('person', people_n, people), ('company', companies_n, companies)]:
            for index in range(count):
                object_id, revision = record(connection, 'entity', f'{kind}/{index}', change_set)
                insert(connection, 'entities', (object_id, kind))
                insert(connection, 'entity_profiles', (revision, object_id, f'{"Personne" if kind == "person" else "Société"} synthétique {index:03d}', None, None, None, 'FR', 'EUR', '', '', 0))
                profile_period(connection, 'entity', object_id, revision, change_set)
                values.append(object_id)
                if kind == 'person':
                    _, membership = record(connection, 'membership', f'membership/{index}', change_set)
                    insert(connection, 'family_memberships', (membership, object_id, '2025-01-01', None, 'Famille synthétique'))
        for index, company in enumerate(companies):
            owner = people[0] if index == 0 else companies[(index - 1) // 2]
            _, revision = record(connection, 'participation', f'participation/{index}', change_set)
            insert(connection, 'company_participations', (revision, owner, company, '1', '1', '1', '1', '1', '1', '2025-01-01', None))
            _, revision = record(connection, 'company_policy', f'policy/{index}', change_set)
            insert(connection, 'company_policies', (revision, company, 'detailed', 1, 1, '2025-01-01', None))
        historical_assets = Decimal(0)
        assets, first_valuation = [], None
        for index in range(assets_n):
            object_id, revision = record(connection, 'asset', f'asset/{index}', change_set)
            insert(connection, 'assets', (object_id,))
            insert(connection, 'asset_profiles', (revision, object_id, f'Actif synthétique {index:05d}', 'FR', 'autre', '', 0))
            profile_period(connection, 'asset', object_id, revision, change_set)
            assets.append(object_id)
            _, ownership = record(connection, 'ownership', f'ownership/{index}', change_set)
            owner = people[0] if index == assets_n - 1 else companies[index % companies_n]
            insert(connection, 'asset_ownerships', (ownership, object_id, owner, '1', '1', '1', '1', '2025-01-01', None))
            if index != assets_n - 1:
                valuation_id, valuation_revision = record(connection, 'valuation', f'valuation/{index}', change_set)
                currency = ('EUR', 'USD', 'TND')[index % 3]
                insert(connection, 'valuations', (valuation_revision, 'asset', object_id, '100', currency, '2025-01-01', 'Oracle de qualification synthétique'))
                historical_assets += (Decimal(100), Decimal(80), Decimal(30))[index % 3]
                if index == 0:
                    first_valuation = (valuation_id, valuation_revision)
        for currency, rate in [('USD', '0.8'), ('TND', '0.3')]:
            for month in range(1, 13):
                _, revision = record(connection, 'exchange_rate', f'rate/{currency}/{month}', change_set)
                insert(connection, 'exchange_rates', (revision, currency, 'EUR', rate, f'2025-{month:02d}-01', 'Cours fixe synthétique de qualification', ''))
        account_id, account_revision = record(connection, 'treasury_account', 'account', change_set)
        insert(connection, 'treasury_accounts', (account_revision, assets[-1], 'bank', 'EUR', '2025-01-01', people[0], 'Banque synthétique', 'BENCHMARK', 'FR', '', '', 0))
        bank_balance = Decimal(100000)
        for index in range(movements_n):
            _, revision = record(connection, 'treasury_transaction', f'transaction/{index}', change_set)
            amount = '100000' if index == 0 else '1' if index % 2 else '-1'
            if index:
                bank_balance += Decimal(amount)
            insert(connection, 'treasury_transactions', (revision, 'opening' if index == 0 else 'correction', '2025-01-01', f'Écriture synthétique {index:06d}', '', 'Ajustement synthétique de qualification, sans source réelle'))
            insert(connection, 'treasury_entries', (revision, 'principal', account_id, amount, 'opening' if index == 0 else 'adjustment'))
        _, revision = record(connection, 'finance_coverage', 'coverage', change_set)
        insert(connection, 'finance_coverages', (revision, '2025-01-01', None, 1, 1, 1, 'Inventaire complet explicitement synthétique'))
        for index in range(documents_n):
            content = (f'Original synthétique de qualification {index:08d}\n'.encode('utf-8') + bytes(128))[:128]
            digest = hashlib.sha256(content).hexdigest()
            relative = f'documents/sha256/{digest[:2]}/{digest}'
            path = generation_path / relative
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            path.write_bytes(content)
            path.chmod(0o600)
            insert(connection, 'document_blobs', (digest, len(content), relative, STAMP))
            document_id, revision = record(connection, 'document', f'document/{index}', change_set)
            insert(connection, 'documents', (document_id,))
            insert(connection, 'document_versions', (revision, digest, f'original-{index:05d}.txt', 'text/plain', 'Document synthétique de qualification de volume', 'active'))
            _, revision = record(connection, 'document_link', f'document-link/{index}', change_set)
            insert(connection, 'document_links', (revision, document_id, assets[index % assets_n], 'Qualification de volume', '2025-01-01', None))
        insert(connection, 'change_sets', (correction, 2, STAMP, 'Correction synthétique conservant le K initial', identity('command/correction')))
        _, revision = record(connection, 'valuation', 'valuation/0', correction, first_valuation[1])
        insert(connection, 'valuations', (revision, 'asset', assets[0], '101', 'EUR', '2025-01-01', 'Correction synthétique, ancien K conservé'))
    assert connection.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    assert not connection.execute('PRAGMA foreign_key_check').fetchall()
    tables = [r[0] for r in connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
    table_counts = {table: connection.execute(f'SELECT count(*) FROM "{table}"').fetchone()[0] for table in tables}
    connection.execute('PRAGMA wal_checkpoint(TRUNCATE)')
    connection.close()
    historical_assets += bank_balance
    expected = {'fixture_version':1, 'profile':profile, 'people':people_n, 'companies':companies_n, 'assets':assets_n,
                'movements':movements_n, 'documents':documents_n, 'unique_blobs':documents_n, 'document_bytes_each':128,
                'dag':'100%-owned binary tree rooted at first family member', 'change_sets':2,
                'seed_semantics':'Bulk initial change and one retrospective valuation correction; not 100,000 sequential commits',
                'table_counts':table_counts, 'rows':sum(table_counts.values()), 'account_id':account_id,
                'owner_id':people[0], 'bank_balance':money(bank_balance), 'historical_assets_eur':money(historical_assets),
                'current_assets_eur':money(historical_assets+1), 'seed_seconds':time.monotonic()-started}
    (vault / 'expected.json').write_text(json.dumps(expected,ensure_ascii=False,indent=2)+'\n')
    return expected


def hardware():
    cpu = next((line.split(':',1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')), 'unknown')
    memory = next((line for line in Path('/proc/meminfo').read_text().splitlines() if line.startswith('MemTotal:')), 'unknown')
    cgroup = {}
    for name in ['cpu.max','memory.max','cpuset.cpus.effective']:
        path = Path('/sys/fs/cgroup') / name
        if path.exists(): cgroup[name] = path.read_text().strip()
    return {'system':platform.platform(), 'machine':platform.machine(), 'cpu_model':cpu, 'logical_cpus':os.cpu_count(),
            'visible_memory':memory, 'cgroup':cgroup, 'clock_ticks_per_second':os.sysconf('SC_CLK_TCK'),
            'sqlite_python':sqlite3.sqlite_version, 'python':platform.python_version(), 'initial_load_average':os.getloadavg()}


def measure(binary, vault, operation, artifacts, timeout):
    started = time.monotonic()
    stdout, stderr = artifacts / f'{operation}.stdout.json', artifacts / f'{operation}.stderr.txt'
    peak_kib = 0
    timed_out = False
    with stdout.open('w') as out, stderr.open('w') as err:
        process = subprocess.Popen([str(binary), str(vault), operation], stdout=out, stderr=err)
        while process.poll() is None:
            status = Path(f'/proc/{process.pid}/status')
            try:
                for line in status.read_text().splitlines():
                    if line.startswith(('VmHWM:', 'VmRSS:')): peak_kib=max(peak_kib,int(line.split()[1]))
            except FileNotFoundError: pass
            if time.monotonic()-started > timeout:
                process.kill(); process.wait(); timed_out=True; break
            time.sleep(0.05)
    record = {'operation':operation, 'exit_code':process.returncode, 'wall_seconds':time.monotonic()-started,
              'sampled_peak_rss_kib':peak_kib, 'timeout_seconds':timeout, 'status':'timeout' if timed_out else 'error'}
    return finish_record(driver_record(record, stdout.read_bytes(), timed_out))


def driver_record(record, raw, timed_out=False):
    """Keep process supervision authoritative over untrusted driver output."""
    record['process_state'] = 'running' if record['exit_code'] is None else 'exited'
    if timed_out:
        return record
    try:
        payload = json.loads(raw)
        if not isinstance(payload, dict) or payload.get('operation') != record['operation']:
            raise ValueError('The driver response must identify the requested operation.')
    except (json.JSONDecodeError, UnicodeDecodeError, ValueError):
        record['invalid_stdout'] = True
        return record
    result = {**payload, **record, 'driver_status': payload.get('status')}
    if payload.get('status') == 'passed':
        result['status'] = 'pending' if record['exit_code'] is None else 'passed' if record['exit_code'] == 0 else 'error'
    return result


def finish_record(record):
    operation = record['operation']
    record['ordinary_consultation_target_seconds']=2 if operation in ['open','registry','treasury','documents','account','financial','individual','historical','fx','dashboard'] else None
    if record['ordinary_consultation_target_seconds'] is not None:
        duration=record.get('open_seconds') if operation=='open' else record.get('operation_seconds')
        record['provisional_target_met']=(record['status']=='passed' and record.get('exit_code') == 0
            and isinstance(duration, (int, float)) and not isinstance(duration, bool)
            and math.isfinite(duration) and 0 <= duration < 2)
    print(json.dumps(record,ensure_ascii=False),flush=True)
    return record


def session_stage(process, operation, artifacts, timeout, stem):
    """Watch one real service call; a timeout kills the entire validated session."""
    started = time.monotonic()
    peak_kib = 0
    data = b''
    timed_out = False
    while True:
        try:
            for line in Path(f'/proc/{process.pid}/status').read_text().splitlines():
                if line.startswith('VmRSS:'): peak_kib=max(peak_kib,int(line.split()[1]))
        except FileNotFoundError: pass
        ready, _, _ = select.select([process.stdout], [], [], 0.05)
        if ready:
            chunk = os.read(process.stdout.fileno(), 65536)
            if not chunk: break
            data += chunk
            if b'\n' in data: break
        if time.monotonic()-started > timeout:
            process.kill(); process.wait(); timed_out=True; break
        if process.poll() is not None and not ready: break
    (artifacts/f'{stem}.stdout.json').write_bytes(data)
    record={'operation':operation, 'exit_code':process.poll(), 'wall_seconds':time.monotonic()-started,
            'sampled_peak_rss_kib':peak_kib, 'timeout_seconds':timeout, 'status':'timeout' if timed_out else 'error',
            'shared_validated_session':True, 'load_average_after':os.getloadavg(),
            'memory_note':'sampled_peak_rss_kib covers this stage; VmHWM_kib is cumulative unless memory_peak_reset is true'}
    return driver_record(record, data, timed_out)


def measure_session(binary, vault, operations, artifacts, timeout):
    pending = list(operations)
    attempt = 0
    while pending:
        attempt += 1
        with (artifacts/f'session-{attempt}.stderr.txt').open('w') as err:
            process=subprocess.Popen([str(binary),str(vault),'--session'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=err)
            records = []
            blocked = None
            shutdown_timed_out = False
            try:
                opened=session_stage(process,'open',artifacts,timeout,f'session-{attempt}-open')
                records.append(opened)
                yield 'startup', opened
                if 'open' in pending:
                    pending.remove('open')
                    yield 'measurement', opened
                if opened.get('driver_status') != 'passed' or opened['status'] not in ('passed', 'pending'):
                    blocked = {'reason':'The real application refused or timed out validating the fixture.','operations':list(pending)}
                while pending and blocked is None:
                    operation=pending.pop(0)
                    try:
                        process.stdin.write((operation+'\n').encode('utf-8')); process.stdin.flush()
                        result=session_stage(process,operation,artifacts,timeout,operation)
                    except BrokenPipeError:
                        result={'operation':operation, 'status':'error', 'exit_code':process.poll(), 'broken_pipe':True}
                    records.append(result)
                    yield 'measurement', result
                    if result['status'] not in ('passed', 'pending') or process.poll() is not None: break
            finally:
                try:
                    process.stdin.close()
                except BrokenPipeError:
                    pass
                try:
                    process.wait(timeout=min(timeout, 5))
                except subprocess.TimeoutExpired:
                    process.kill(); process.wait(); shutdown_timed_out=True
                process.stdout.close()
            for result in records:
                result['exit_code'] = process.returncode
                result['process_state'] = 'exited'
                if shutdown_timed_out:
                    result['status'] = 'timeout'
                    result['shutdown_timeout'] = True
                elif result['status'] != 'timeout':
                    result['status'] = 'passed' if process.returncode == 0 and result.get('driver_status') == 'passed' else 'error'
                finish_record(result)
            yield 'session_exit', {'attempt':attempt, 'exit_code':process.returncode,
                                  'status':'passed' if process.returncode == 0 and not shutdown_timed_out else 'error',
                                  'shutdown_timeout':shutdown_timed_out}
            if blocked is not None:
                yield 'blocked', blocked
                return


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--profile',choices=PROFILES,default='pilot')
    parser.add_argument('--binary',type=Path,default=ROOT/'target/release/examples/qualify_performance')
    parser.add_argument('--operations',nargs='+',choices=OPERATIONS,default=OPERATIONS)
    parser.add_argument('--timeout',type=float,default=120)
    parser.add_argument('--reuse',action='store_true')
    parser.add_argument('--seed-only',action='store_true')
    parser.add_argument('--session',action='store_true',help='Validate once per process and measure calls sequentially; kill/reopen after timeout.')
    args=parser.parse_args()
    output=args.output.resolve(); vault=output/'vault'; binary=args.binary.resolve()
    if not binary.is_file(): parser.error('Construire d’abord le driver Rust de qualification en release.')
    if not args.reuse:
        output.mkdir(mode=0o700,parents=True,exist_ok=False)
        subprocess.run([str(binary),str(vault),'init'],check=True,capture_output=True,text=True)
        expected=seed(vault,args.profile)
    else:
        expected=json.loads((vault/'expected.json').read_text())
        if expected['profile']!=args.profile: parser.error('Le profil demandé diffère du coffre réutilisé.')
    metadata={'created_at':datetime.now(timezone.utc).isoformat(), 'profile':args.profile, 'hardware':hardware(),
              'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(), 'binary':str(binary),
              'release_profile_required':True, 'fixture':expected, 'measurements':[],
              'qualification_limit':'Synthetic volume with two change sets. Per-operation single samples, no GUI latency claim, disk caches not cleared; startup validation measured separately.',
              'measurement_mode':'shared validated process; per-operation watchdog, CPU deltas and sampled RSS; Linux VmHWM reset attempted for each call' if args.session else 'isolated process per operation',
              'concurrent_load_caveat':'Other repository checks/builds may run concurrently; observed load averages are recorded, results are not controlled-machine benchmarks.'}
    report=output/'performance.json'
    report.write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n')
    if args.seed_only: print(json.dumps({'status':'seeded','fixture':expected},ensure_ascii=False)); return 0
    if args.session:
        metadata['session_startups']=[]
        metadata['session_processes']=[]
        for kind,result in measure_session(binary,vault,args.operations,output,args.timeout):
            if kind=='startup': metadata['session_startups'].append(result)
            elif kind=='measurement': metadata['measurements'].append(result)
            elif kind=='session_exit': metadata['session_processes'].append(result)
            else: metadata['remaining_operations_skipped']=result
            metadata['hardware']['last_load_average']=os.getloadavg()
            report.write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n')
        return 0 if (not metadata.get('remaining_operations_skipped')
            and len(metadata['measurements']) == len(args.operations)
            and all(r['status']=='passed' for r in metadata['session_startups'] + metadata['measurements'] + metadata['session_processes'])) else 1
    for operation in args.operations:
        result=measure(binary,vault,operation,output,args.timeout)
        metadata['measurements'].append(result)
        metadata['hardware']['last_load_average']=os.getloadavg()
        report.write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n')
        if operation=='open' and result['status']!='passed':
            metadata['remaining_operations_skipped']='The real application refused or timed out validating the fixture.'
            report.write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n'); break
    return 0 if all(r['status']=='passed' for r in metadata['measurements']) else 1


if __name__=='__main__':
    raise SystemExit(main())
