#!/usr/bin/env bash

info() {
  printf '[writing-environment] %s\n' "$*"
}

fail() {
  printf '[writing-environment] Error: %s\n' "$*" >&2
  exit 1
}

require_regular_user() {
  [[ "${EUID:-$(id -u)}" -ne 0 ]] || fail "Run this script as your normal desktop user, not with sudo. It requests sudo only for system files."
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "Required command not found: $1"
}

check_supported_host() {
  local allow_unsupported="$1"
  local architecture
  architecture="$(uname -m)"

  if [[ "$architecture" != "x86_64" ]]; then
    if [[ "$allow_unsupported" == "1" ]]; then
      info "Warning: continuing on unsupported architecture $architecture."
    else
      fail "This port targets x86-64 Linux; detected $architecture. Use --allow-unsupported only for development checks."
    fi
  fi

  [[ -r /etc/os-release ]] || fail "Cannot identify the operating system."
  # shellcheck source=/dev/null
  source /etc/os-release
  case "${ID:-}:${ID_LIKE:-}" in
    *debian*) ;;
    *)
      if [[ "$allow_unsupported" == "1" ]]; then
        info "Warning: continuing on a non-Debian Linux distribution."
      else
        fail "The native installer currently supports Debian and Ubuntu x86-64. Use the AppImage on other distributions."
      fi
      ;;
  esac
}

enable_xdg_autostart() {
  local source_entry="$1"
  local target="${XDG_CONFIG_HOME:-$HOME/.config}/autostart/writing-environment.desktop"
  mkdir -p "$(dirname "$target")"
  install -m 644 "$source_entry" "$target"
}

disable_xdg_autostart() {
  local target="${XDG_CONFIG_HOME:-$HOME/.config}/autostart/writing-environment.desktop"
  [[ -f "$target" ]] || return 0
  if grep -Fxq 'X-Writing-Environment-Managed=true' "$target"; then
    rm -f -- "$target"
  else
    info "Preserved user-managed autostart file: $target"
  fi
}
