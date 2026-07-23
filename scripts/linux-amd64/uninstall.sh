#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$script_dir/lib.sh"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/linux-amd64/uninstall.sh

Remove the installed program, launcher, icon, and managed XDG autostart entry.
Projects, Markdown files, recovery data, configuration, packages, and developer
toolchains are deliberately preserved.
EOF
  exit 0
fi
[[ $# -eq 0 ]] || fail "This command takes no arguments."
require_regular_user

disable_xdg_autostart
info "Removing installed application files. Your writing and recovery data will not be touched."
sudo rm -f -- \
  /opt/writing-environment/writing-environment \
  /usr/local/bin/writing-environment \
  /usr/local/share/applications/writing-environment.desktop \
  /usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg
sudo rmdir /opt/writing-environment 2>/dev/null || true

if command -v update-desktop-database >/dev/null 2>&1; then
  sudo update-desktop-database /usr/local/share/applications >/dev/null 2>&1 || true
fi

info "Uninstalled. Projects, recovery data, and ~/.config/writing-environment were preserved."
