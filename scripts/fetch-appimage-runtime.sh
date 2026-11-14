#!/usr/bin/env bash
# Runtime officiel de sortie AppImage, vérifié avant usage par appimagetool.
set -euo pipefail
cd "$(dirname "$0")/.."
runtime_sha=617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46
runtime_url=https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64
runtime_dir="${XDG_CACHE_HOME:-$PWD/.local/build-cache}/tauri"
runtime_path="$runtime_dir/runtime-x86_64-$runtime_sha"
mkdir -p "$runtime_dir"
if ! test -f "$runtime_path"; then
  temporary=$(mktemp "$runtime_dir/.runtime.XXXXXXXX")
  trap 'rm -f "$temporary"' EXIT
  curl --fail --location --show-error --retry 2 --max-time 60 "$runtime_url" -o "$temporary"
  actual=$(sha256sum "$temporary")
  if [[ "${actual%% *}" != "$runtime_sha" ]]; then
    echo 'Le runtime AppImage téléchargé diffère de la version épinglée ; examiner la nouvelle version avant de modifier son empreinte.' >&2
    exit 1
  fi
  chmod 600 "$temporary"
  mv "$temporary" "$runtime_path"
  trap - EXIT
fi
actual=$(sha256sum "$runtime_path")
[[ "${actual%% *}" == "$runtime_sha" ]] || { echo 'Le cache du runtime AppImage est altéré.' >&2; exit 1; }
printf '%s\n' "$runtime_path"
