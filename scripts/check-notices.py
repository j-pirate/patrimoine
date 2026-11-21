#!/usr/bin/env python3
"""Validate generated notices and their exact lockfiles/final native inventory."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate(notices, root=ROOT, appdir=None, embedded=None):
    manifest = json.loads((notices / 'manifest.json').read_text())
    if manifest.get('generated_by') != 'scripts/collect-third-party-licences.py':
        raise ValueError('Unknown notices generator')
    for field, filename in [('cargo_lock_sha256', 'Cargo.lock'), ('package_lock_sha256', 'package-lock.json')]:
        if manifest.get(field) != sha256(root / filename):
            raise ValueError(f'Stale notices: {filename} changed')
    if manifest.get('application_licence') != 'GPL-3.0-only':
        raise ValueError('Application licence must be GPL-3.0-only')
    if manifest.get('missing_licence_files') or manifest.get('unmapped_bundled_libraries'):
        raise ValueError('Notices have missing texts or unidentified native libraries')
    application = manifest.get('application_licence_file', {})
    if application.get('sha256') != sha256(root / 'LICENSE'):
        raise ValueError('Application licence text changed or is absent')
    entries = [application, *manifest.get('debian_common_licence_files', [])]
    for package in manifest['packages']:
        if not package.get('files'):
            raise ValueError(f'Missing licence text: {package["name"]}')
        entries.extend(package['files'])
        if package.get('ecosystem') in ('debian', 'ubuntu'):
            if (package.get('package_environment') not in ('host', 'sysroot')
                    or package.get('package_environment') != package.get('copyright_environment')):
                raise ValueError(f'Native binary/copyright origin differs or is absent: {package["name"]}')
    for entry in entries:
        path = notices / entry['path']
        if not path.resolve().is_relative_to(notices.resolve()) or not path.is_file() or sha256(path) != entry['sha256']:
            raise ValueError(f'Licence file hash/path mismatch: {entry["path"]}')
    if appdir is not None:
        actual = [{'path': path.relative_to(appdir).as_posix(), 'sha256': sha256(path)}
                  for path in sorted(appdir.rglob('*')) if path.is_file() and '.so' in path.name]
        if not manifest.get('appdir_scanned') or manifest.get('appdir_bundled_libraries') != actual:
            raise ValueError('Final AppDir native inventory differs from collected notices')
    if embedded is not None:
        for path in notices.rglob('*'):
            if path.is_file():
                target = embedded / path.relative_to(notices)
                if not target.is_file() or sha256(target) != sha256(path):
                    raise ValueError(f'AppDir contains stale notices: {path.relative_to(notices)}')
    return len(manifest['packages'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--notices', type=Path, default=ROOT / 'resources/notices')
    parser.add_argument('--appdir', type=Path)
    parser.add_argument('--embedded', type=Path)
    args = parser.parse_args()
    try:
        count = validate(args.notices, appdir=args.appdir, embedded=args.embedded)
    except (ValueError, KeyError, OSError) as error:
        parser.exit(1, f'Notices refused: {error}\n')
    print(f'Notices verified: {count} packages; lockfiles and licence hashes match.')


if __name__ == '__main__':
    main()
