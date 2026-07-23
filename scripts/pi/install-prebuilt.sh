#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly script_dir
# The artifact packager copies lib.sh beside this installer.
# shellcheck disable=SC1091
source "$script_dir/lib.sh"

skip_packages=0
autostart_action="enable"
allow_unsupported=0

usage() {
  cat <<'EOF'
Usage: ./install.sh [options]

Install the prebuilt Writing Environment binary from this extracted artifact.

Options:
  --skip-packages       Do not install or update runtime dependencies
  --no-autostart        Install the app but disable its Labwc autostart block
  --preserve-autostart  Leave the current Labwc autostart file unchanged
  --allow-unsupported   Permit a non-Pi/non-ARM host for development checks
  -h, --help            Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-packages) skip_packages=1 ;;
    --no-autostart) autostart_action="disable" ;;
    --preserve-autostart) autostart_action="preserve" ;;
    --allow-unsupported) allow_unsupported=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; fail "Unknown option: $1" ;;
  esac
  shift
done

require_regular_user
check_supported_host "$allow_unsupported"
require_command sha256sum
require_command file

readonly manifest="$script_dir/manifest.txt"
readonly checksums="$script_dir/SHA256SUMS"
readonly bundled_binary="$script_dir/writing-environment"
for required_file in \
  "$manifest" \
  "$checksums" \
  "$bundled_binary" \
  "$script_dir/launcher" \
  "$script_dir/writing-environment.desktop" \
  "$script_dir/environment.example" \
  "$script_dir/app-icon.svg"; do
  [[ -f "$required_file" && ! -L "$required_file" ]] || fail "Artifact file is missing or unsafe: $required_file"
done

grep -qx 'format=1' "$manifest" || fail "Unsupported artifact manifest format."
grep -qx 'architecture=aarch64' "$manifest" || fail "This is not a Raspberry Pi ARM64 artifact."
(
  cd "$script_dir"
  sha256sum -c SHA256SUMS
) || fail "Artifact checksum verification failed."
file "$bundled_binary" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' \
  || fail "The bundled executable is not Linux ARM64."

if [[ "$skip_packages" == "0" ]]; then
  info "Installing Raspberry Pi runtime dependencies."
  sudo apt-get update
  sudo apt-get install -y \
    ca-certificates \
    curl \
    file \
    hunspell-en-us \
    hunspell-pt-br \
    libayatana-appindicator3-1 \
    librsvg2-2 \
    libwebkit2gtk-4.1-0 \
    libxdo3 \
    unzip \
    util-linux

  if ! command -v labwc >/dev/null 2>&1; then
    info "No Labwc session detected; installing the Raspberry Pi Wayland desktop core."
    sudo apt-get install -y rpd-wayland-core rpd-theme rpd-preferences
  fi
  install_compatible_rclone
fi

require_command flock
missing_libraries="$(ldd "$bundled_binary" | awk '/not found/ { print $1 }')"
[[ -z "$missing_libraries" ]] || fail "Runtime libraries are missing: $missing_libraries"

info "Installing the verified prebuilt application."
# The bootstrap executable is extracted from Tauri's Debian bundle, so it
# remembers that future signed updates should be installed with dpkg. Keeping
# it at the Debian package's final path also makes the first self-update and
# relaunch seamless.
sudo install -Dm755 "$bundled_binary" /usr/bin/writing-environment
sudo install -Dm755 "$script_dir/launcher" /usr/local/bin/writing-environment
sudo install -Dm644 "$script_dir/writing-environment.desktop" /usr/local/share/applications/writing-environment.desktop
sudo install -Dm644 "$script_dir/app-icon.svg" /usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg

readonly config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/writing-environment"
mkdir -p "$config_dir"
if [[ ! -e "$config_dir/environment" ]]; then
  install -m 644 "$script_dir/environment.example" "$config_dir/environment"
fi

readonly autostart_file="${XDG_CONFIG_HOME:-$HOME/.config}/labwc/autostart"
case "$autostart_action" in
  enable)
    enable_autostart "$autostart_file"
    info "Enabled launch at Labwc login."
    ;;
  disable)
    remove_autostart_block "$autostart_file"
    info "Labwc autostart is disabled."
    ;;
  preserve)
    info "Left the existing Labwc autostart setting unchanged."
    ;;
esac

if command -v update-desktop-database >/dev/null 2>&1; then
  sudo update-desktop-database /usr/local/share/applications >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  sudo gtk-update-icon-cache -q -t /usr/local/share/icons/hicolor >/dev/null 2>&1 || true
fi

info "Installation complete. Start it now with: writing-environment"
