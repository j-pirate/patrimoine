#!/usr/bin/env python3
"""Qualification locale de deux AppImages et de leurs données XDG synthétiques.

À exécuter dans l'image Debian de test, sans réseau ni outils de compilation.
Les paquets fournis sont des candidats locaux, pas des versions déjà publiées.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import sqlite3
import subprocess
import tempfile
import time


def module(path, name):
    specification = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(loaded)
    return loaded


def digest(path):
    result = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            result.update(block)
    return result.hexdigest()


def database_state(vault):
    generation = (vault / 'CURRENT').read_text().strip()
    folder = vault / 'generations' / generation
    with sqlite3.connect('file:' + str(folder / 'patrimoine.sqlite3') + '?mode=ro', uri=True) as connection:
        assert connection.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        assert not connection.execute('PRAGMA foreign_key_check').fetchall()
        tables = [row[0] for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
        assert len(tables) == 62
        hashes = {}
        counts = {}
        for table in tables:
            rows = sorted(json.dumps(list(row), ensure_ascii=False, separators=(',', ':')) for row in connection.execute(f'SELECT * FROM "{table}"'))
            hashes[table] = hashlib.sha256('\n'.join(rows).encode()).hexdigest()
            counts[table] = len(rows)
        version = connection.execute('PRAGMA user_version').fetchone()[0]
    originals = {file.relative_to(folder).as_posix(): digest(file) for file in (folder / 'documents').rglob('*') if file.is_file()}
    return {'generation': generation, 'schema': version, 'tables': hashes, 'counts': counts, 'originals': originals}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--previous', required=True, type=Path)
    parser.add_argument('--current', required=True, type=Path)
    parser.add_argument('--backup', required=True, type=Path)
    parser.add_argument('--artifacts', required=True, type=Path)
    parser.add_argument('--previous-version', default='0.1.0')
    parser.add_argument('--current-version', default='0.1.1')
    parser.add_argument('--driver-module', type=Path, default=Path(__file__).with_name('desktop-smoke.py'))
    parser.add_argument('--fixture-module', type=Path, default=Path(__file__).with_name('audit-desktop-accessibility.py'))
    args = parser.parse_args()
    args.artifacts.mkdir(parents=True, exist_ok=True)
    driver_module = module(args.driver_module, 'native_driver')
    fixture_module = module(args.fixture_module, 'fixture_loader')
    report = {'status': 'failed', 'started_at': datetime.now(timezone.utc).isoformat(), 'steps': [], 'candidate_versions': [args.previous_version, args.current_version], 'network': 'disabled by Docker runner'}
    process = None
    driver = None
    logfile = None
    try:
        absent = {tool: shutil.which(tool) for tool in ['rustc', 'cargo', 'node', 'npm', 'gcc', 'g++']}
        assert not any(absent.values()), absent
        report['development_tools'] = absent
        report['appimages'] = {'previous_sha256': digest(args.previous), 'current_sha256': digest(args.current)}
        assert report['appimages']['previous_sha256'] != report['appimages']['current_sha256']
        with tempfile.TemporaryDirectory(prefix='patrimoine-appimage-upgrade-') as temporary:
            root = Path(temporary)
            xdg = root / 'user-data'
            vault = xdg / 'patrimoine'
            fixture = fixture_module.load_synthetic_copy(args.backup, vault)
            source_k = fixture['mutation_sequence']
            baseline = database_state(vault)
            assert baseline['schema'] == 6 and len(baseline['originals']) == 2
            installed = root / 'installation' / 'Patrimoine.AppImage'
            installed.parent.mkdir(mode=0o700)
            environment = {**os.environ, 'XDG_DATA_HOME': str(xdg), 'APPIMAGE_EXTRACT_AND_RUN': '1'}
            environment.pop('PATRIMOINE_DATA_DIR', None)

            def install(source):
                replacement = installed.with_suffix('.new')
                shutil.copyfile(source, replacement)
                replacement.chmod(0o700)
                replacement.replace(installed)
                assert digest(installed) == digest(source)

            def stop():
                nonlocal driver, process, logfile
                if driver:
                    try:
                        driver.close()
                    finally:
                        driver = None
                if process:
                    try:
                        os.killpg(process.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait(timeout=5)
                    process = None
                if logfile:
                    logfile.close()
                    logfile = None

            def start(label, version, sequence):
                nonlocal driver, process, logfile
                started = time.monotonic()
                logfile = (args.artifacts / f'{label}-driver.log').open('w')
                driver = driver_module.WebDriver()
                process = subprocess.Popen(['tauri-driver', '--port', '4444', '--native-port', '4445'], env=environment, stdout=logfile, stderr=subprocess.STDOUT, start_new_session=True)
                driver_module.wait_until(lambda: driver.request('GET', '/status', timeout=2), 'pilote natif', timeout=20)
                driver.start(installed)
                driver_module.wait_until(lambda: driver.execute("return !!window.__TAURI_INTERNALS__ && document.querySelector('h1')?.textContent==='Tableau de bord';"), 'fenêtre AppImage', timeout=45)
                status = driver.invoke('get_status')
                assert status['app_version'] == version, status
                assert status['schema_version'] == 6 and status['mutation_sequence'] == sequence
                assert status['data_dir'] == str(vault), status
                driver_module.wait_text(driver, 'Accessible', timeout=30)
                report['steps'].append({'name': label, 'version': version, 'sequence': sequence, 'startup_seconds': round(time.monotonic() - started, 3), 'xdg_data_dir': status['data_dir'], 'status': 'passed'})
                return status

            commands = ['list_registry_at', 'list_properties_at', 'list_rentals_at', 'list_treasury_at', 'list_debts_at', 'list_finance_at', 'list_metals_at', 'list_companies_at', 'list_document_library_at']

            def source_reports():
                snapshots = {command: driver.invoke(command, {'knowledgeSequence': source_k}) for command in commands}
                snapshots['financial'] = driver.invoke('get_financial_report', {'request': {'context': 'family', 'person_id': None, 'date': '2026-11-12', 'knowledge_sequence': source_k}})
                return snapshots

            install(args.previous)
            previous = start('previous-installation', args.previous_version, source_k)
            expected_reports = source_reports()
            assert database_state(vault) == baseline
            driver.screenshot(args.artifacts / 'previous-installation.png')
            stop()
            install(args.current)
            current = start('updated-installation', args.current_version, source_k)
            assert current['generation_id'] == previous['generation_id']
            assert source_reports() == expected_reports
            assert database_state(vault) == baseline
            driver_module.save_settings_ui(driver, 'Installation mise à jour v' + args.current_version, 'CHF')
            changed = database_state(vault)
            assert changed['schema'] == 6 and changed['generation'] == baseline['generation'] and changed['originals'] == baseline['originals']
            for table in baseline['tables']:
                if table not in ['app_settings', 'change_sets', 'audit_events']:
                    assert changed['tables'][table] == baseline['tables'][table], table
            assert changed['counts']['change_sets'] == baseline['counts']['change_sets'] + 1
            assert changed['counts']['audit_events'] == baseline['counts']['audit_events'] + 1
            assert source_reports() == expected_reports
            report['steps'].append({'name': 'new-version-write-retains-source-knowledge-and-originals', 'status': 'passed', 'sequence': source_k + 1})
            driver.screenshot(args.artifacts / 'updated-installation.png')
            stop()
            # Restart the actual new package before replacing it with the old one.
            restarted = start('updated-offline-restart', args.current_version, source_k + 1)
            assert restarted['settings']['workspace_name'] == 'Installation mise à jour v' + args.current_version
            assert database_state(vault) == changed and source_reports() == expected_reports
            stop()
            install(args.previous)
            rolled_back = start('previous-version-rollback', args.previous_version, source_k + 1)
            assert rolled_back['settings'] == restarted['settings']
            assert database_state(vault) == changed and source_reports() == expected_reports
            driver.screenshot(args.artifacts / 'previous-version-rollback.png')
            stop()
            # A real newer schema is refused before application startup.
            future_xdg = root / 'future-user-data'
            shutil.copytree(xdg, future_xdg)
            future_vault = future_xdg / 'patrimoine'
            future_db = future_vault / 'generations' / changed['generation'] / 'patrimoine.sqlite3'
            with sqlite3.connect(future_db) as connection:
                connection.execute('PRAGMA user_version=7')
            future_before = database_state(future_vault)
            rejected = subprocess.run([str(installed)], env={**environment, 'XDG_DATA_HOME': str(future_xdg)}, capture_output=True, text=True, timeout=30)
            assert rejected.returncode != 0 and 'Version de base non prise en charge : 7' in rejected.stderr, rejected.stderr
            assert database_state(future_vault) == future_before
            report['steps'].append({'name': 'previous-version-refuses-future-schema-without-data-loss', 'status': 'passed', 'exit_code': rejected.returncode})
            report['source_sequence'] = source_k
            report['updated_sequence'] = source_k + 1
            report['source_tables'] = 62
            report['source_rows'] = sum(baseline['counts'].values())
            report['originals_preserved'] = len(baseline['originals'])
            report['source_table_hashes'] = baseline['tables']
            report['status'] = 'passed'
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
        if driver and driver.session_id:
            try:
                (args.artifacts / 'failure.txt').write_text(driver.text())
                driver.screenshot(args.artifacts / 'failure.png')
            except Exception:
                pass
    finally:
        if process:
            try:
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=5)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                pass
        if logfile:
            logfile.close()
        report['completed_at'] = datetime.now(timezone.utc).isoformat()
        (args.artifacts / 'appimage-upgrade-result.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
        print(json.dumps({key: value for key, value in report.items() if key != 'source_table_hashes'}, ensure_ascii=False, indent=2))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
