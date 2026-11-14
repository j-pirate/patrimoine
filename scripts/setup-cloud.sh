#!/usr/bin/env bash
# Préparation non privilégiée de l'environnement cloud Debian 13 x86_64.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATRIMOINE_TOOL_ROOT="${PATRIMOINE_TOOL_ROOT:-/workspace/.tooling}"
mkdir -p "$PATRIMOINE_TOOL_ROOT"
source scripts/cloud-env.sh

rust_version=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml)
if ! command -v rustup >/dev/null; then
  rustup_url=https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
  curl --fail --show-error --location --retry 2 "$rustup_url" -o "$PATRIMOINE_TOOL_ROOT/rustup-init"
  curl --fail --show-error --location --retry 2 "$rustup_url.sha256" -o "$PATRIMOINE_TOOL_ROOT/rustup-init.sha256"
  python3 - "$PATRIMOINE_TOOL_ROOT" <<'PY'
import hashlib, pathlib, sys
p = pathlib.Path(sys.argv[1])
expected = (p / 'rustup-init.sha256').read_text().split()[0]
if hashlib.sha256((p / 'rustup-init').read_bytes()).hexdigest() != expected:
    raise SystemExit('Échec du checksum officiel de rustup-init')
(p / 'rustup-init').chmod(0o700)
PY
  "$PATRIMOINE_TOOL_ROOT/rustup-init" -y --no-modify-path --profile minimal --default-toolchain "$rust_version" --component rustfmt --component clippy
else
  rustup toolchain install "$rust_version" --profile minimal --component rustfmt --component clippy
fi

if ! pkg-config --exists webkit2gtk-4.1 gtk+-3.0; then
  if [ "$(uname -m)" != x86_64 ] || ! test -f /usr/share/keyrings/debian-archive-keyring.gpg; then
    echo 'Ce script cloud cible Debian 13 x86_64. Voir docs/07-developpement.md pour une installation Linux classique.' >&2
    exit 1
  fi
  if ! . /etc/os-release || [ "${ID:-}" != debian ] || [ "${VERSION_ID:-}" != 13 ]; then
    echo 'Distribution non validée pour le sysroot cloud : Debian 13 requis.' >&2
    exit 1
  fi
  mkdir -p "$PATRIMOINE_TOOL_ROOT/apt/lists/partial" "$PATRIMOINE_TOOL_ROOT/apt/archives/partial" "$PATRIMOINE_TOOL_ROOT/apt/empty" "$PATRIMOINE_TOOL_ROOT/sysroot"
  cat > "$PATRIMOINE_TOOL_ROOT/apt.conf" <<EOF
Dir::Etc::main "/dev/null";
Dir::Etc::parts "$PATRIMOINE_TOOL_ROOT/apt/empty";
Dir::Etc::sourcelist "$PATRIMOINE_TOOL_ROOT/apt/sources.list";
Dir::Etc::sourceparts "$PATRIMOINE_TOOL_ROOT/apt/empty";
Dir::State::lists "$PATRIMOINE_TOOL_ROOT/apt/lists";
Dir::Cache::archives "$PATRIMOINE_TOOL_ROOT/apt/archives";
Acquire::Retries "2";
EOF
  cat > "$PATRIMOINE_TOOL_ROOT/apt/sources.list" <<'EOF'
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://deb.debian.org/debian trixie main
deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://deb.debian.org/debian-security trixie-security main
EOF
  APT_CONFIG="$PATRIMOINE_TOOL_ROOT/apt.conf" /usr/bin/apt-get update
  APT_CONFIG="$PATRIMOINE_TOOL_ROOT/apt.conf" /usr/bin/apt-get --download-only -y -o Debug::NoLocking=1 --no-install-recommends install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev patchelf xvfb xauth dbus-x11
  python3 - "$PATRIMOINE_TOOL_ROOT" <<'PY'
import pathlib, subprocess, sys
p = pathlib.Path(sys.argv[1])
for package in sorted((p / 'apt/archives').glob('*.deb')):
    subprocess.run(['dpkg-deb', '-x', str(package), str(p / 'sysroot')], check=True)
PY
fi
python3 - "$PATRIMOINE_TOOL_ROOT" <<'PY'
import pathlib, sys
root = pathlib.Path(sys.argv[1]) / 'sysroot'
# Les paquets -dev peuvent viser un runtime déjà installé sur l'hôte : APT ne
# le retélécharge pas. Réutiliser ces fichiers système vérifiés, sans les copier.
if (root / 'usr/lib').exists():
    for link in list((root / 'usr/lib').rglob('*')):
        if not link.is_symlink() or link.exists():
            continue
        target = link.readlink()
        target = link.parent / target if not target.is_absolute() else root / str(target).lstrip('/')
        try:
            host = pathlib.Path('/') / target.relative_to(root)
        except ValueError:
            continue
        if host.exists() and not target.exists() and not target.is_symlink():
            target.parent.mkdir(parents=True, exist_ok=True)
            target.symlink_to(host)
PY
source scripts/cloud-env.sh
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1
npm ci --no-audit --no-fund
cargo fetch --locked
echo 'Outils et dépendances préparés. Sourcer scripts/cloud-env.sh avant les commandes Rust/Tauri.'
