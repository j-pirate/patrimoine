#!/usr/bin/env python3
"""Run the real packaging recipe against fake offline bundlers and licence inputs."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parent


class PackagingGateTests(unittest.TestCase):
    def build(self, scenario):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        for name in ('scripts', 'bin', 'src-tauri', 'resources/notices'):
            (root / name).mkdir(parents=True)
        for filename in ('build-linux.sh', 'check-notices.py', 'source-identity.py'):
            shutil.copyfile(SCRIPTS / filename, root / 'scripts' / filename)
        for filename in ('LICENSE', 'Cargo.lock', 'package-lock.json'):
            (root / filename).write_text(filename)
        (root / 'src-tauri/tauri.conf.json').write_text('{"productName":"Patrimoine","version":"0.1.1"}')
        (root / 'scripts/prepare-appimage-runtime.py').write_text('from pathlib import Path\nPath(".local/build/appimage-runtime-manifest.json").write_text("[]")\nprint("runtime.json")\n')
        (root / 'scripts/fetch-appimage-runtime.sh').write_text('touch .local/runtime\nprintf "%s\\n" "$PWD/.local/runtime"\n')
        (root / 'scripts/collect-third-party-licences.py').write_text('''import hashlib,json,sys
from pathlib import Path
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
out=Path('resources/notices'); out.mkdir(exist_ok=True)
appdir=Path(sys.argv[sys.argv.index('--appdir')+1]) if '--appdir' in sys.argv else None
(out/'APPLICATION-LICENSE.txt').write_bytes(Path('LICENSE').read_bytes())
data={'generated_by':'scripts/collect-third-party-licences.py', 'application_licence':'GPL-3.0-only',
'cargo_lock_sha256':sha(Path('Cargo.lock')), 'package_lock_sha256':sha(Path('package-lock.json')),
'application_licence_file':{'path':'APPLICATION-LICENSE.txt','sha256':sha(Path('LICENSE'))},
'packages':[], 'appdir_scanned':appdir is not None,
'appdir_bundled_libraries':[{'path':p.relative_to(appdir).as_posix(),'sha256':sha(p)} for p in sorted(appdir.rglob('*')) if p.is_file() and '.so' in p.name] if appdir else []}
(out/'manifest.json').write_text(json.dumps(data))
''')
        command = root / 'bin/tool'
        command.write_text('''#!/usr/bin/env python3
import os,shutil,sys
from pathlib import Path
tool=Path(sys.argv[0]).name
if tool=='npm' and 'bundle' in sys.argv:
    count=Path('.local/bundle-count'); n=int(count.read_text())+1 if count.exists() else 1; count.write_text(str(n))
    app=Path('target/release/bundle/appimage/Patrimoine.AppDir'); (app/'usr/lib/Patrimoine').mkdir(parents=True,exist_ok=True)
    (app/'libfixture.so').write_text('changed' if n==2 and os.environ['SCENARIO']=='native' else 'native')
    if not (n==2 and os.environ['SCENARIO']=='embedded'):
        shutil.copytree('resources/notices',app/'usr/lib/Patrimoine/notices',dirs_exist_ok=True)
    (app.parent/'Patrimoine_0.1.1_amd64.AppImage').write_text('fixture')
elif '--version' in sys.argv: print('fixture version')
''')
        command.chmod(0o700)
        for name in ('node', 'npm', 'cargo', 'rustc', 'pkg-config', 'patchelf'):
            (root / 'bin' / name).symlink_to('tool')
        env = {**os.environ, 'PATH': str(root / 'bin') + ':' + os.environ['PATH'], 'SCENARIO': scenario}
        result = subprocess.run(['bash', 'scripts/build-linux.sh'], cwd=root, env=env, capture_output=True, text=True)
        return result, root

    def test_two_pass_build_embeds_final_notices_and_calculates_source_identity(self):
        result, root = self.build('success')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((root / '.local/bundle-count').read_text(), '2')
        manifest = json.loads((root / 'target/release/bundle/appimage/build-manifest.json').read_text())
        self.assertTrue(manifest['application_sources_unchanged'])
        self.assertIn('source_sha256', manifest['source_identity'])
        self.assertIn('notices_manifest_sha256', manifest)

    def test_final_native_or_embedded_changes_refuse_build_manifest(self):
        for scenario in ('native', 'embedded'):
            with self.subTest(scenario=scenario):
                result, root = self.build(scenario)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('Notices refused', result.stderr)
                self.assertFalse((root / 'target/release/bundle/appimage/build-manifest.json').exists())


if __name__ == '__main__':
    unittest.main()
