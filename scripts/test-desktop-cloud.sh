#!/usr/bin/env bash
# E2E réel dans Debian : aucun réseau ni donnée utilisateur pendant l'exécution.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/cloud-env.sh
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
if [[ $# -gt 1 ]]; then
  echo 'Usage : bash scripts/test-desktop-cloud.sh [binaire-ou-AppImage-construit]' >&2
  exit 2
fi
if [[ $# -eq 1 ]]; then
  application=$(realpath "$1")
  test -x "$application"
else
  npm run build
  cargo build -p patrimoine-desktop --features custom-protocol --locked
  application="$PWD/target/debug/patrimoine"
fi
if ! test -x "$PATRIMOINE_TOOL_ROOT/driver/bin/tauri-driver"; then
  cargo install tauri-driver --version 2.1.0 --locked --root "$PATRIMOINE_TOOL_ROOT/driver"
fi

docker_local() {
  env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
    docker --host=unix:///var/run/docker.sock "$@"
}
docker_local info >/dev/null
docker_local build --secret id=system_ca,src=/etc/ssl/certs/ca-certificates.crt \
  -f scripts/desktop-test.Dockerfile -t patrimoine-desktop-test:phase2 .
mkdir -p .local
artifacts=$(mktemp -d "$PWD/.local/desktop-smoke.XXXXXXXX")
echo "Rapport et captures : $artifacts"
python3 - "$application" "$artifacts" <<'PY'
from pathlib import Path
import hashlib, json, sys
application, artifacts = map(Path, sys.argv[1:])
proof = {'application_input': str(application), 'size_bytes': application.stat().st_size,
         'sha256': hashlib.sha256(application.read_bytes()).hexdigest(),
         'launch_mode': 'APPIMAGE_EXTRACT_AND_RUN=1; also accepted by an ordinary ELF binary'}
(artifacts / 'application-under-test.json').write_text(json.dumps(proof, indent=2) + '\n')
PY
docker_local run --rm --network none --shm-size=512m \
  -e APPIMAGE_EXTRACT_AND_RUN=1 \
  --mount "type=bind,src=$application,dst=/app/patrimoine,readonly" \
  --mount "type=bind,src=$PATRIMOINE_TOOL_ROOT/driver/bin/tauri-driver,dst=/usr/local/bin/tauri-driver,readonly" \
  --mount "type=bind,src=$PWD/scripts/desktop-smoke.py,dst=/app/desktop-smoke.py,readonly" \
  --mount "type=bind,src=$artifacts,dst=/artifacts" \
  patrimoine-desktop-test:phase2 sh -c '
    Xvfb :99 -screen 0 1280x1024x24 -nolisten tcp >/tmp/xvfb.log 2>&1 &
    export DISPLAY=:99
    exec dbus-run-session -- python3 /app/desktop-smoke.py /app/patrimoine --artifacts /artifacts --native-dialogs
  '
