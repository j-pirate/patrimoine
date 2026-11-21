#!/usr/bin/env python3
"""Summarize a local RustSec report; require review of new/expired warnings.

This never contacts a server or uploads the dependency inventory. Full local
reports retain warnings. The CI summary contains database coverage and warning
identifiers only, without pretending a warning is a fixed dependency.
"""
import argparse
from datetime import date
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def review(report, policy, today=None):
    known = {(item['id'], item['package'], item['version'], item['kind']) for item in policy['tracked_warnings']}
    warnings = [(item['advisory']['id'], item['package']['name'], item['package']['version'], kind)
                for kind, items in report['warnings'].items() for item in items]
    unexpected = [entry for entry in warnings if entry not in known]
    expired = (today or date.today()) >= date.fromisoformat(policy['review_again_before'])
    return warnings, unexpected, expired


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rust-report', type=Path, required=True)
    parser.add_argument('--database', type=Path, required=True)
    parser.add_argument('--policy', type=Path, default=ROOT / '.github/advisory-review.json')
    args = parser.parse_args()
    report = json.loads(args.rust_report.read_text())
    policy = json.loads(args.policy.read_text())
    warnings, unexpected, expired = review(report, policy)
    def git(*arguments):
        return subprocess.check_output(['git', '-C', str(args.database), *arguments], text=True).strip()
    summary = {'database_repository': 'https://github.com/RustSec/advisory-db', 'database_commit': git('rev-parse', 'HEAD'),
               'database_commit_time': git('show', '-s', '--format=%cI', 'HEAD'),
               'database_advisory_count': report['database']['advisory-count'],
               'locked_dependencies': report['lockfile']['dependency-count'],
               'cargo_lock_sha256': hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest(),
               'vulnerabilities': report['vulnerabilities']['count'], 'warnings': warnings,
               'reviewed_at': policy['reviewed_at'], 'review_again_before': policy['review_again_before'],
               'coverage': 'RustSec lockfile comparison; yanked-registry checks excluded to avoid inventory-dependent network requests',
               'review_scope': policy['scope'], 'unexpected_warnings': unexpected, 'review_expired': expired}
    text = json.dumps(summary, ensure_ascii=False, indent=2)
    args.rust_report.with_name('rust-advisory-review.json').write_text(text + '\n')
    print(text)
    if os.environ.get('GITHUB_STEP_SUMMARY'):
        with open(os.environ['GITHUB_STEP_SUMMARY'], 'a') as output:
            output.write('RustSec: local comparison with fresh public database.\n\n```json\n' + text + '\n```\n')
    return 1 if report['vulnerabilities']['count'] or unexpected or expired else 0


if __name__ == '__main__':
    raise SystemExit(main())
