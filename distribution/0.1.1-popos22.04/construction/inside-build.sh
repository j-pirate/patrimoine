#!/usr/bin/env bash
set -euo pipefail
cd /work
unset PKG_CONFIG_SYSROOT_DIR PKG_CONFIG_PATH LD_LIBRARY_PATH PATRIMOINE_TOOL_ROOT
export CARGO_NET_OFFLINE=true
export CARGO_BUILD_JOBS=3
export XDG_CACHE_HOME=/work/.local/build-cache
export APPIMAGE_EXTRACT_AND_RUN=1

node --version
rustc --version
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1
bash scripts/build-linux.sh

mkdir -p /tmp/patrimoine-native-only/sysroot
python3 scripts/collect-third-party-licences.py \
  --sysroot /tmp/patrimoine-native-only/sysroot \
  --appdir target/release/bundle/appimage/Patrimoine.AppDir

python3 - <<'PY'
import json
from pathlib import Path
p=Path('resources/notices/manifest.json')
d=json.loads(p.read_text())
assert not d['missing_licence_files'], d['missing_licence_files']
assert not d['unmapped_bundled_libraries'], d['unmapped_bundled_libraries']
assert any(x['ecosystem']=='ubuntu' for x in d['packages'])
print('Ubuntu native notices verified:',len(d['packages']),'components')
PY

bash scripts/build-linux.sh
python3 - <<'PY'
from pathlib import Path
import hashlib,json,platform,subprocess
base=Path('target/release/bundle')
artifacts=[]
for p in sorted(base.glob('*/*')):
    if p.is_file() and p.suffix == '.AppImage':
        artifacts.append({'file':str(p),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
assert len(artifacts)==1,artifacts
native=subprocess.check_output(['dpkg-query','-W','-f=${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\n'],text=True)
manifest={'build_os':platform.freedesktop_os_release(),'glibc':platform.libc_ver(),'artifacts':artifacts,'native_packages':native.splitlines(),'application_sources_unchanged':True,'notes':'Packaging-only isolated-copy adaptations; no changes to application calculations or schema.'}
Path('.local/pop-os22-build-result.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'status':'built','artifacts':artifacts}))
PY
