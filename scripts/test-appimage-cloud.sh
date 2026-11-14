#!/usr/bin/env bash
# Installation/mise à jour/retour en arrière sur des données exclusivement synthétiques.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/cloud-env.sh
if [[ $# -ne 3 ]]; then
  echo 'Usage : bash scripts/test-appimage-cloud.sh ancien.AppImage nouveau.AppImage sauvegarde-synthetique.zip' >&2
  exit 2
fi
previous=$(realpath "$1")
current=$(realpath "$2")
fixture=$(realpath "$3")
test -x "$previous" && test -x "$current" && test -f "$fixture"
docker_local() {
  env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
    docker --host=unix:///var/run/docker.sock "$@"
}
docker_local info >/dev/null
docker_local build --secret id=system_ca,src=/etc/ssl/certs/ca-certificates.crt -f scripts/desktop-test.Dockerfile -t patrimoine-desktop-test:phase2 .
mkdir -p .local
artifacts=$(mktemp -d "$PWD/.local/appimage-upgrade.XXXXXXXX")
echo "Rapport AppImage : $artifacts"
docker_local run --rm --network none --shm-size=512m \
  --mount "type=bind,src=$previous,dst=/app/previous.AppImage,readonly" \
  --mount "type=bind,src=$current,dst=/app/current.AppImage,readonly" \
  --mount "type=bind,src=$fixture,dst=/app/fixture.zip,readonly" \
  --mount "type=bind,src=$PATRIMOINE_TOOL_ROOT/driver/bin/tauri-driver,dst=/usr/local/bin/tauri-driver,readonly" \
  --mount "type=bind,src=$PWD/scripts,dst=/app/scripts,readonly" \
  --mount "type=bind,src=$artifacts,dst=/artifacts" \
  patrimoine-desktop-test:phase2 sh -c '
    Xvfb :99 -screen 0 1280x1024x24 -nolisten tcp >/tmp/xvfb.log 2>&1 &
    export DISPLAY=:99
    exec dbus-run-session -- python3 /app/scripts/test-appimage-upgrade.py \
      --previous /app/previous.AppImage --current /app/current.AppImage --backup /app/fixture.zip --artifacts /artifacts
  '
