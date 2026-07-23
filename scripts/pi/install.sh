#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
# shellcheck source=lib.sh
source "$script_dir/lib.sh"

skip_packages=0
autostart_action="enable"
allow_unsupported=0

usage() {
  cat <<'EOF'
Usage: scripts/pi/install.sh [options]

Build and install Writing Environment natively on a 64-bit Raspberry Pi 4.

Options:
  --skip-packages       Do not install or update Debian build dependencies
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

[[ -f "$repo_root/package.json" ]] || fail "Run this installer from a complete Writing Environment checkout."
[[ -f "$repo_root/pnpm-lock.yaml" ]] || fail "pnpm-lock.yaml is missing."
case "$repo_root/" in
  */.local/share/Trash/*)
    fail "This checkout is inside the desktop Trash: $repo_root. Extract or restore it to a normal folder such as $HOME/writing-environment, then run the installer there."
    ;;
esac

if [[ "$skip_packages" == "0" ]]; then
  info "Installing Raspberry Pi OS and Tauri dependencies."
  sudo apt-get update
  sudo apt-get install -y \
    build-essential \
    curl \
    file \
    git \
    hunspell-en-us \
    hunspell-pt-br \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev \
    nodejs \
    npm \
    unzip \
    util-linux \
    wget

  if ! command -v labwc >/dev/null 2>&1; then
    info "No Labwc session detected; installing the Raspberry Pi Wayland desktop core."
    sudo apt-get install -y rpd-wayland-core rpd-theme rpd-preferences
  fi
  install_compatible_rclone
fi

require_command node
node_major="$(node -p 'Number(process.versions.node.split(".")[0])')"
[[ "$node_major" -ge 20 ]] || fail "Node.js 20 or newer is required; detected $(node --version)."

pnpm_major=0
if command -v pnpm >/dev/null 2>&1; then
  pnpm_major="$(pnpm --version | cut -d. -f1)"
fi
if [[ ! "$pnpm_major" =~ ^[0-9]+$ ]] || [[ "$pnpm_major" -lt 10 ]]; then
  require_command npm
  info "Installing pnpm 10."
  sudo npm install --global pnpm@10
fi

if ! command -v cargo >/dev/null 2>&1; then
  require_command curl
  info "Installing the stable Rust toolchain with rustup."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  export PATH="$HOME/.cargo/bin:$PATH"
fi

require_command cargo
require_command pnpm
require_command flock

readonly cargo_target_dir="$repo_root/src-tauri/target"
readonly source_marker="$cargo_target_dir/.writing-environment-source"
cached_source=""
if [[ -r "$source_marker" ]]; then
  cached_source="$(<"$source_marker")"
fi
if [[ -d "$cargo_target_dir" ]] && [[ "$cached_source" != "$repo_root" ]]; then
  info "Clearing a Cargo cache created before this checkout path was recorded."
  CARGO_TARGET_DIR="$cargo_target_dir" cargo clean --manifest-path "$repo_root/src-tauri/Cargo.toml"
fi
mkdir -p "$cargo_target_dir"
printf '%s\n' "$repo_root" >"$source_marker"

info "Building the optimized native ARM64 application. This can take several minutes on a Pi 4."
(
  cd "$repo_root"
  export CARGO_TARGET_DIR="$cargo_target_dir"
  pnpm install --frozen-lockfile
  pnpm tauri build --bundles deb --config '{"bundle":{"createUpdaterArtifacts":false}}'
)

readonly built_deb="$(find "$repo_root/src-tauri/target/release/bundle/deb" -maxdepth 1 -type f -name '*.deb' -print -quit)"
[[ -f "$built_deb" ]] || fail "The ARM64 Debian bundle was not created."
deb_root="$(mktemp -d)"
trap 'rm -rf "$deb_root"' EXIT
dpkg-deb -x "$built_deb" "$deb_root"
readonly built_binary="$deb_root/usr/bin/writing-environment"
[[ -x "$built_binary" ]] || fail "The Debian bundle does not contain its executable."

info "Installing the application and desktop launcher."
sudo install -Dm755 "$built_binary" /usr/bin/writing-environment
sudo install -Dm755 "$repo_root/deploy/pi/writing-environment" /usr/local/bin/writing-environment
sudo install -Dm644 "$repo_root/deploy/pi/writing-environment.desktop" /usr/local/share/applications/writing-environment.desktop
sudo install -Dm644 "$repo_root/design/app-icon.svg" /usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg

readonly config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/writing-environment"
mkdir -p "$config_dir"
if [[ ! -e "$config_dir/environment" ]]; then
  install -m 644 "$repo_root/deploy/pi/environment.example" "$config_dir/environment"
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
info "It will open automatically after the next Labwc login unless --no-autostart was used."
