#!/usr/bin/env bash
# Audit du vrai WebKit, sur copie d'une sauvegarde synthétique explicite.
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo 'Usage : bash scripts/audit-desktop-cloud.sh sauvegarde-synthetique.zip [executable-ou-AppImage]' >&2
  exit 2
fi
source scripts/cloud-env.sh
audit_backup=$(realpath "$1")
if [ "$#" -eq 2 ]; then
  audit_application=$(realpath "$2")
else
  npm run build
  CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}" cargo build -p patrimoine-desktop --features custom-protocol --locked
  audit_application="$PWD/target/debug/patrimoine"
fi
test -f "$audit_backup"
test -x "$audit_application"
if ! test -f node_modules/axe-core/axe.min.js; then
  echo 'Installer les dépendances de développement avec npm ci avant cet audit.' >&2
  exit 2
fi
docker_local() {
  env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY -u DOCKER_CERT_PATH \
    docker --host=unix:///var/run/docker.sock "$@"
}
docker_local info >/dev/null
docker_local build --secret id=system_ca,src=/etc/ssl/certs/ca-certificates.crt \
  -f scripts/desktop-test.Dockerfile -t patrimoine-desktop-test:phase2 .
mkdir -p .local
audit_artifacts=$(mktemp -d "$PWD/.local/accessibility.XXXXXXXX")
echo "Audit et captures : $audit_artifacts"
docker_local run --rm --network none --shm-size=512m \
  -e APPIMAGE_EXTRACT_AND_RUN=1 \
  --mount "type=bind,src=$audit_application,dst=/app/patrimoine,readonly" \
  --mount "type=bind,src=$PATRIMOINE_TOOL_ROOT/driver/bin/tauri-driver,dst=/usr/local/bin/tauri-driver,readonly" \
  --mount "type=bind,src=$PWD/scripts,dst=/app/scripts,readonly" \
  --mount "type=bind,src=$PWD/node_modules/axe-core/axe.min.js,dst=/app/axe.min.js,readonly" \
  --mount "type=bind,src=$audit_backup,dst=/app/synthetic-backup.zip,readonly" \
  --mount "type=bind,src=$audit_artifacts,dst=/artifacts" \
  patrimoine-desktop-test:phase2 sh -c '
    Xvfb :99 -screen 0 1280x1024x24 -nolisten tcp >/tmp/xvfb.log 2>&1 &
    export DISPLAY=:99
    exec dbus-run-session -- python3 /app/scripts/audit-desktop-accessibility.py /app/patrimoine \
      --backup /app/synthetic-backup.zip --axe /app/axe.min.js --artifacts /artifacts
  '
