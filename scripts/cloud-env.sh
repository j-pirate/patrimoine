#!/usr/bin/env bash
# À sourcer : outils locaux du cloud Debian, hors dépôt et hors données utilisateur.
export PATRIMOINE_TOOL_ROOT="${PATRIMOINE_TOOL_ROOT:-/workspace/.tooling}"
export CARGO_HOME="$PATRIMOINE_TOOL_ROOT/cargo"
export RUSTUP_HOME="$PATRIMOINE_TOOL_ROOT/rustup"
patrimoine_prepend_path() {
  local variable_name="$1" entry="$2" existing="${!1-}"
  case ":$existing:" in
    *":$entry:"*) ;;
    *) printf -v "$variable_name" '%s' "$entry${existing:+:$existing}"; export "$variable_name" ;;
  esac
}
patrimoine_prepend_path PATH "$PATRIMOINE_TOOL_ROOT/sysroot/usr/bin"
patrimoine_prepend_path PATH "$CARGO_HOME/bin"
if [ -d "$PATRIMOINE_TOOL_ROOT/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig" ]; then
  export PKG_CONFIG_SYSROOT_DIR="$PATRIMOINE_TOOL_ROOT/sysroot"
  patrimoine_prepend_path PKG_CONFIG_PATH "$PATRIMOINE_TOOL_ROOT/sysroot/usr/share/pkgconfig"
  patrimoine_prepend_path PKG_CONFIG_PATH "$PATRIMOINE_TOOL_ROOT/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig"
  patrimoine_prepend_path LD_LIBRARY_PATH "$PATRIMOINE_TOOL_ROOT/sysroot/usr/lib/x86_64-linux-gnu"
  export XDG_DATA_DIRS="${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
  patrimoine_prepend_path XDG_DATA_DIRS "$PATRIMOINE_TOOL_ROOT/sysroot/usr/share"
fi
unset -f patrimoine_prepend_path
