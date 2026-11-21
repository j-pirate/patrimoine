#!/usr/bin/env bash
# Current native Ubuntu 22.04 recipe; historical distribution recipes stay archived.
set -euo pipefail
cd "$(dirname "$0")/.."
source /etc/os-release
[[ "$ID" == ubuntu && "$VERSION_ID" == 22.04 ]] || { echo 'Ubuntu 22.04 requis.' >&2; exit 1; }
unset PKG_CONFIG_SYSROOT_DIR PKG_CONFIG_PATH LD_LIBRARY_PATH PATRIMOINE_TOOL_ROOT
export CARGO_NET_OFFLINE=true
export PATRIMOINE_TARGET_DISTRIBUTION='Pop!_OS 22.04 / Ubuntu 22.04 x86_64 (glibc 2.35)'
# Native host packages alone must account for the bundled runtime.
export PATRIMOINE_NOTICES_SYSROOT="$PWD/.local/ubuntu-native-only/sysroot"
mkdir -p "$PATRIMOINE_NOTICES_SYSROOT"
bash scripts/build-linux.sh
