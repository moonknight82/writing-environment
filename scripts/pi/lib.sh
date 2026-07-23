#!/usr/bin/env bash

readonly WE_AUTOSTART_BEGIN="# BEGIN WRITING ENVIRONMENT AUTOSTART"
readonly WE_AUTOSTART_END="# END WRITING ENVIRONMENT AUTOSTART"

info() {
  printf '[writing-environment] %s\n' "$*"
}

fail() {
  printf '[writing-environment] Error: %s\n' "$*" >&2
  exit 1
}

require_regular_user() {
  [[ "${EUID:-$(id -u)}" -ne 0 ]] || fail "Run this script as your normal desktop user, not with sudo. It will request sudo only for system files."
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "Required command not found: $1"
}

rclone_is_compatible() {
  command -v rclone >/dev/null 2>&1 || return 1
  local version
  local major
  local minor
  version="$(rclone version 2>/dev/null | head -n 1)"
  version="${version#rclone v}"
  major="${version%%.*}"
  version="${version#*.}"
  minor="${version%%.*}"
  [[ "$major" =~ ^[0-9]+$ && "$minor" =~ ^[0-9]+$ ]] || return 1
  (( major > 1 || (major == 1 && minor >= 66) ))
}

install_compatible_rclone() {
  if rclone_is_compatible; then
    info "Compatible $(rclone version | head -n 1) is already installed."
    return 0
  fi

  require_command curl
  require_command unzip
  local installer_directory
  local installer
  installer_directory="$(mktemp -d)"
  installer="$installer_directory/rclone-install.sh"
  info "Installing a current rclone release from rclone.org (version 1.66 or newer is required)."
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
    https://rclone.org/install.sh \
    --output "$installer"
  sudo bash "$installer" || fail "The official rclone installer did not complete."
  rm -rf "$installer_directory"
  rclone_is_compatible || fail "rclone 1.66 or newer is required for safe two-way sync."
}

is_raspberry_pi_4() {
  [[ -r /proc/device-tree/model ]] || return 1
  tr -d '\0' </proc/device-tree/model | grep -q 'Raspberry Pi 4'
}

check_supported_host() {
  local allow_unsupported="$1"
  local architecture
  architecture="$(uname -m)"

  if [[ "$architecture" != "aarch64" ]]; then
    if [[ "$allow_unsupported" == "1" ]]; then
      info "Warning: continuing on unsupported architecture $architecture."
    else
      fail "This kit targets 64-bit Raspberry Pi OS (aarch64); detected $architecture. Use --allow-unsupported only for development checks."
    fi
  fi

  if ! is_raspberry_pi_4; then
    if [[ "$allow_unsupported" == "1" ]]; then
      info "Warning: Raspberry Pi 4 hardware was not detected."
    else
      fail "Raspberry Pi 4 hardware was not detected. Use --allow-unsupported only for a compatible ARM64 build host."
    fi
  fi

  [[ -r /etc/os-release ]] || fail "Cannot identify the operating system."
  # shellcheck source=/dev/null
  source /etc/os-release
  case "${ID:-}:${ID_LIKE:-}" in
    *debian*) ;;
    *)
      if [[ "$allow_unsupported" == "1" ]]; then
        info "Warning: this does not appear to be a Debian-family system."
      else
        fail "This installer supports 64-bit Raspberry Pi OS/Debian."
      fi
      ;;
  esac
}

remove_autostart_block() {
  local target_file="$1"
  local temporary
  local begin_count
  local end_count
  [[ -f "$target_file" ]] || return 0

  begin_count="$(grep -Fxc "$WE_AUTOSTART_BEGIN" "$target_file" || true)"
  end_count="$(grep -Fxc "$WE_AUTOSTART_END" "$target_file" || true)"
  if [[ "$begin_count" -ne "$end_count" ]]; then
    fail "The managed block in $target_file is incomplete. Restore its BEGIN/END markers before continuing."
  fi

  temporary="$(mktemp "${target_file}.XXXXXX")"
  awk -v begin="$WE_AUTOSTART_BEGIN" -v end="$WE_AUTOSTART_END" '
    $0 == begin { skipping = 1; next }
    $0 == end { skipping = 0; next }
    !skipping { print }
  ' "$target_file" >"$temporary"
  if ! chmod --reference="$target_file" "$temporary" 2>/dev/null; then
    chmod "$(stat -f '%Lp' "$target_file")" "$temporary"
  fi
  mv "$temporary" "$target_file"
}

enable_autostart() {
  local target_file="$1"
  mkdir -p "$(dirname "$target_file")"

  if [[ ! -e "$target_file" ]]; then
    install -m 644 /dev/null "$target_file"
  elif [[ ! -f "$target_file" ]]; then
    fail "Labwc autostart path is not a regular file: $target_file"
  fi

  remove_autostart_block "$target_file"
  if [[ -s "$target_file" ]] && [[ "$(tail -c 1 "$target_file" | wc -l)" -eq 0 ]]; then
    printf '\n' >>"$target_file"
  fi
  {
    printf '%s\n' "$WE_AUTOSTART_BEGIN"
    printf '/usr/local/bin/writing-environment &\n'
    printf '%s\n' "$WE_AUTOSTART_END"
  } >>"$target_file"
}
