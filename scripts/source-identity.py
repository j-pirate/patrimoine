#!/usr/bin/env python3
"""Fingerprint application/build inputs, with Git identity when available.

Generated notices, test reports, documentation and historical distributions are
outside this source fingerprint. Notice and lockfile hashes are recorded by the
build manifest separately. This also works in an exported source directory.
"""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DIRECTORIES = ('src', 'crates', 'src-tauri', 'scripts', 'resources/licence-sources', '.github')
ROOT_FILES = ('Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json', 'rust-toolchain.toml',
              '.node-version', 'index.html', 'vite.config.ts', 'tsconfig.json', 'tsconfig.app.json',
              'tsconfig.node.json', 'LICENSE')
EXCLUDED_PARTS = {'target', 'node_modules', 'gen', '__pycache__'}


def capture(root=ROOT):
    paths = {root / name for name in ROOT_FILES if (root / name).is_file()}
    for name in DIRECTORIES:
        paths.update(path for path in (root / name).rglob('*')
                     if (path.is_file() or path.is_symlink())
                     and not EXCLUDED_PARTS.intersection(path.relative_to(root).parts))
    files = []
    for path in sorted(paths):
        raw = path.readlink().as_posix().encode() if path.is_symlink() else path.read_bytes()
        files.append({'path': path.relative_to(root).as_posix(), 'kind': 'symlink' if path.is_symlink() else 'file',
                      'executable': bool(path.lstat().st_mode & 0o111), 'sha256': hashlib.sha256(raw).hexdigest()})

    def git(*arguments):
        try:
            result = subprocess.run(['git', '-C', str(root), *arguments], capture_output=True)
        except FileNotFoundError:
            return None
        return result.stdout.decode().rstrip('\n') if result.returncode == 0 else None

    status = git('status', '--porcelain=v1', '--untracked-files=all')
    return {'source_commit': git('rev-parse', 'HEAD'), 'source_git_tree': git('rev-parse', 'HEAD^{tree}'),
            'git_worktree_dirty': bool(status) if status is not None else None,
            'git_status': status.splitlines() if status else [],
            'source_sha256': hashlib.sha256(json.dumps(files, sort_keys=True, separators=(',', ':')).encode()).hexdigest(),
            'source_file_count': len(files), 'source_files': files,
            'scope': {'directories': DIRECTORIES, 'root_files': ROOT_FILES,
                      'excludes': sorted(EXCLUDED_PARTS), 'generated_notices_excluded': True}}


if __name__ == '__main__':
    print(json.dumps(capture(), ensure_ascii=False, indent=2))
