#!/usr/bin/env python3
"""Check source identity in a Git checkout and an exported source tree."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('source_identity', Path(__file__).with_name('source-identity.py'))
identity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(identity)


class SourceIdentityTests(unittest.TestCase):
    def test_fingerprint_tracks_inputs_and_ignores_generated_outputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'src').mkdir()
            (root / 'src/app.ts').write_text('original')
            before = identity.capture(root)
            self.assertIsNone(before['source_commit'])
            (root / 'resources/notices').mkdir(parents=True)
            (root / 'resources/notices/manifest.json').write_text('generated')
            self.assertEqual(before['source_sha256'], identity.capture(root)['source_sha256'])
            (root / 'src/app.ts').write_text('changed')
            self.assertNotEqual(before['source_sha256'], identity.capture(root)['source_sha256'])

    def test_commit_dirty_and_untracked_input_are_recorded(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / 'Cargo.toml').write_text('original')
            subprocess.run(['git', '-C', str(root), 'add', '.'], check=True)
            subprocess.run(['git', '-C', str(root), '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                            'commit', '-qm', 'fixture'], check=True)
            before = identity.capture(root)
            self.assertRegex(before['source_commit'], r'^[0-9a-f]{40}$')
            self.assertFalse(before['git_worktree_dirty'])
            (root / 'src').mkdir()
            (root / 'src/new.ts').write_text('new input')
            after = identity.capture(root)
            self.assertTrue(after['git_worktree_dirty'])
            self.assertNotEqual(before['source_sha256'], after['source_sha256'])
            self.assertIn('src/new.ts', [entry['path'] for entry in after['source_files']])


if __name__ == '__main__':
    unittest.main()
