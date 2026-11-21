#!/usr/bin/env python3
"""Synthetic native provenance and build-gate regressions; no host writes."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


def module(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f'{name}.py'))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


collector = module('collect-third-party-licences')
checker = module('check-notices')


class NativeOriginTests(unittest.TestCase):
    def inventory(self, same=False, host_notice=True, archive_matches=True, ubuntu=False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        sysroot, host, appdir, stage = [root / name for name in ('sysroot', 'host', 'AppDir', 'notices')]
        archive = root / 'apt/archives/old.deb'
        archive.parent.mkdir(parents=True)
        archive.touch()
        relative = 'usr/lib/libfixture.so.1'
        for directory, content in [(sysroot, b'current' if same else b'old'), (host, b'current'), (appdir, b'current')]:
            path = directory / relative
            path.parent.mkdir(parents=True)
            path.write_bytes(content)
        for directory, content in [(sysroot, b'old notice'), (host, b'current notice')]:
            if directory == host and not host_notice:
                continue
            path = directory / 'usr/share/doc/libfixture/copyright'
            path.parent.mkdir(parents=True)
            path.write_bytes(content)
        if ubuntu:
            (host / 'etc').mkdir()
            (host / 'etc/os-release').write_text('ID=ubuntu\n')

        def output(command, **kwargs):
            if command[0] == 'dpkg-deb':
                if command[1] == '--extract':
                    directory = Path(command[-1])
                    for original in sysroot.rglob('*'):
                        if original.is_file():
                            target = directory / original.relative_to(sysroot)
                            target.parent.mkdir(parents=True, exist_ok=True)
                            target.write_bytes(original.read_bytes())
                    if not archive_matches:
                        (directory / relative).write_bytes(b'other archive version')
                    return b''
                if command[1] == '--contents':
                    return '-rwxr-xr-x root/root 7 2026-01-01 00:00 ./usr/lib/libfixture.so.1\n'
                return {'Package': 'libfixture', 'Version': '1.0', 'Source': 'fixture (1.0)'}[command[-1]]
            return '2.0\tfixture\t2.0'

        def run(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, 'libfixture:amd64: /usr/lib/libfixture.so.1\n' if command[0] == 'dpkg-query' else '', '')

        with patch.object(collector.subprocess, 'check_output', side_effect=output), patch.object(collector.subprocess, 'run', side_effect=run):
            records, unmapped, _ = collector.debian_inventory(stage, appdir, sysroot, host)
        self.assertFalse(unmapped)
        return records[0], stage

    def test_stale_sysroot_does_not_mask_matching_host(self):
        record, stage = self.inventory()
        self.assertEqual(record['version'], '2.0')
        self.assertEqual(record['package_environment'], 'host')
        self.assertEqual(record['copyright_environment'], 'host')
        self.assertEqual((stage / record['files'][0]['path']).read_bytes(), b'current notice')

    def test_matching_sysroot_keeps_its_own_notice_and_version(self):
        record, stage = self.inventory(same=True)
        self.assertEqual(record['version'], '1.0')
        self.assertEqual(record['package_environment'], 'sysroot')
        self.assertEqual((stage / record['files'][0]['path']).read_bytes(), b'old notice')

    def test_missing_host_notice_is_not_filled_from_stale_sysroot(self):
        record, _ = self.inventory(host_notice=False)
        self.assertEqual(record['package_environment'], 'host')
        self.assertEqual(record['files'], [])

    def test_cached_archive_version_must_match_the_sysroot_bytes(self):
        record, stage = self.inventory(same=True, archive_matches=False)
        self.assertEqual(record['package_environment'], 'host')
        self.assertEqual(record['version'], '2.0')
        self.assertEqual((stage / record['files'][0]['path']).read_bytes(), b'current notice')

    def test_ubuntu_host_has_ubuntu_source_provenance(self):
        record, _ = self.inventory(ubuntu=True)
        self.assertEqual(record['ecosystem'], 'ubuntu')
        self.assertEqual(record['source_browser_url'], 'https://launchpad.net/ubuntu/+source/fixture/2.0')


class NoticeGateTests(unittest.TestCase):
    def test_lock_native_and_embedded_changes_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            notices, appdir, embedded = [root / name for name in ('notices', 'AppDir', 'embedded')]
            for directory in (notices, appdir, embedded):
                directory.mkdir()
            for filename in ('Cargo.lock', 'package-lock.json', 'LICENSE'):
                (root / filename).write_text(filename)
            (notices / 'APPLICATION-LICENSE.txt').write_bytes((root / 'LICENSE').read_bytes())
            (appdir / 'libtest.so').write_bytes(b'native')
            manifest = {'generated_by': collector.MARKER, 'application_licence': 'GPL-3.0-only',
                'cargo_lock_sha256': checker.sha256(root / 'Cargo.lock'), 'package_lock_sha256': checker.sha256(root / 'package-lock.json'),
                'application_licence_file': {'path':'APPLICATION-LICENSE.txt', 'sha256':checker.sha256(root / 'LICENSE')},
                'packages': [], 'appdir_scanned': True, 'appdir_bundled_libraries': collector.bundled_libraries(appdir)}
            (notices / 'manifest.json').write_text(json.dumps(manifest))
            for path in notices.iterdir():
                (embedded / path.name).write_bytes(path.read_bytes())
            checker.validate(notices, root, appdir, embedded)
            for path in (root / 'Cargo.lock', appdir / 'libtest.so', embedded / 'APPLICATION-LICENSE.txt'):
                previous = path.read_bytes()
                path.write_bytes(b'changed')
                with self.assertRaises(ValueError):
                    checker.validate(notices, root, appdir, embedded)
                path.write_bytes(previous)
            manifest['missing_licence_files'] = ['cargo:missing@1']
            (notices / 'manifest.json').write_text(json.dumps(manifest))
            with self.assertRaises(ValueError):
                checker.validate(notices, root)


if __name__ == '__main__':
    unittest.main()
