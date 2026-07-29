#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
# shellcheck disable=SC1091
source "$script_dir/lib.sh"

usage() {
  cat <<'EOF'
Usage: ./scripts/pi/update-current-appliance-from-source.sh

Build the application natively on a Raspberry Pi 4 and update an existing
Writing Environment appliance with the desktop shell, keyboard, and boot
splash contained in this source kit.

Run this as the normal desktop user, not with sudo. An internet connection is
required for Raspberry Pi OS packages and the application build dependencies.
EOF
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  "") ;;
  *) usage >&2; exit 2 ;;
esac

require_regular_user
check_supported_host 0

printf '%s\n' \
  'This will build the current application and update the Pi appliance.' \
  'Writing projects and application preferences will be preserved.' \
  'Make sure important manuscripts have finished syncing before continuing.'

if [[ -t 0 ]]; then
  read -r -p 'Continue? [y/N] ' answer
  case "$answer" in
    y|Y|yes|YES) ;;
    *) printf 'Update cancelled.\n'; exit 0 ;;
  esac
fi

"$script_dir/build-on-pi.sh"

archive="$(
  find "$repo_root/artifacts/pi-arm64" -type f \
    -name 'writing-environment-pi-arm64-*.tar.gz' -print \
    | LC_ALL=C sort \
    | tail -n 1
)"
[[ -f "$archive" ]] || fail "The native build completed without producing an application archive."

checksum="$archive.sha256"
if [[ -f "$checksum" ]]; then
  (
    cd "$(dirname "$archive")"
    sha256sum -c "$(basename "$checksum")"
  )
fi

"$script_dir/apply-appliance-update.sh" "$archive"

printf '\nThe update is installed. Reboot when ready with:\n  sudo reboot\n'
