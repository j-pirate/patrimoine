#!/usr/bin/env bash
# Construire le paquet local ; ne publie et ne téléverse aucun artefact.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  echo 'La cible de livraison qualifiée est Linux x86_64. Voir le guide de livraison.' >&2
  exit 1
fi
for tool in node npm cargo rustc pkg-config patchelf; do
  command -v "$tool" >/dev/null || { echo "Outil requis absent : $tool" >&2; exit 1; }
done
pkg-config --exists gtk+-3.0 webkit2gtk-4.1

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
# Les AppImages des outils de packaging peuvent fonctionner sans montage FUSE.
export APPIMAGE_EXTRACT_AND_RUN=1
export XDG_CACHE_HOME="${PATRIMOINE_BUILD_CACHE:-$PWD/.local/build-cache}"
mkdir -p "$XDG_CACHE_HOME"
npm run tauri -- build --ci --no-bundle -- --locked
runtime_config=$(python3 scripts/prepare-appimage-runtime.py)
export LDAI_RUNTIME_FILE
LDAI_RUNTIME_FILE=$(bash scripts/fetch-appimage-runtime.sh)
# Le sysroot sert à compiler ; les plugins GTK doivent copier les ressources
# réelles de /usr et non les chemins du sysroot inscrits par pkg-config.
env -u PKG_CONFIG_SYSROOT_DIR npm run tauri -- bundle --ci --bundles appimage --config "$runtime_config"

python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import platform
import os
import subprocess

root = Path.cwd()
config = json.loads((root / 'src-tauri/tauri.conf.json').read_text())
folder = root / 'target/release/bundle/appimage'
artifact = folder / f"{config['productName']}_{config['version']}_amd64.AppImage"
artifacts = [artifact]
if not artifact.is_file():
    raise SystemExit('La construction n’a produit aucune AppImage.')
manifest = {
    'product': config['productName'],
    'version': config['version'],
    'architecture': platform.machine(),
    'target_distribution': 'Debian 13 x86_64',
    'build_distribution': platform.freedesktop_os_release().get('PRETTY_NAME', platform.platform()),
    'build_os': {key: platform.freedesktop_os_release().get(key) for key in ('ID', 'VERSION_ID', 'PRETTY_NAME')},
    'build_kernel': platform.release(),
    'build_glibc': dict(zip(('implementation', 'version'), platform.libc_ver())),
    'schema_version': 6,
    'rust': subprocess.check_output(['rustc', '--version'], text=True).strip(),
    'node': subprocess.check_output(['node', '--version'], text=True).strip(),
    'cargo_lock_sha256': hashlib.sha256((root / 'Cargo.lock').read_bytes()).hexdigest(),
    'npm_lock_sha256': hashlib.sha256((root / 'package-lock.json').read_bytes()).hexdigest(),
    'webkit_helpers': json.loads((root / '.local/build/appimage-runtime-manifest.json').read_text()),
    'appimage_runtime': {'source': 'https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64', 'sha256': hashlib.sha256(Path(os.environ['LDAI_RUNTIME_FILE']).read_bytes()).hexdigest()},
    'reproducibility': 'Dépendances verrouillées et composants WebKit empreintés ; identité binaire entre deux constructions non revendiquée.',
    'artifacts': [],
}
checksums = []
for artifact in artifacts:
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
    manifest['artifacts'].append({'file': artifact.name, 'size_bytes': artifact.stat().st_size, 'sha256': digest})
    checksums.append(f'{digest}  {artifact.name}\n')
(folder / 'SHA256SUMS').write_text(''.join(checksums), encoding='ascii')
(folder / 'build-manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(folder)
PY
