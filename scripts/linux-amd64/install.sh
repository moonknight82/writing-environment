#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
# shellcheck source=lib.sh
source "$script_dir/lib.sh"

skip_packages=0
autostart=0
allow_unsupported=0

usage() {
  cat <<'EOF'
Usage: scripts/linux-amd64/install.sh [options]

Build and install Writing Environment on Debian/Ubuntu x86-64.

Options:
  --skip-packages      Do not install or update Debian build dependencies
  --autostart          Open Writing Environment after desktop login
  --allow-unsupported  Permit another host for development checks
  -h, --help           Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-packages) skip_packages=1 ;;
    --autostart) autostart=1 ;;
    --allow-unsupported) allow_unsupported=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; fail "Unknown option: $1" ;;
  esac
  shift
done

require_regular_user
check_supported_host "$allow_unsupported"
[[ -f "$repo_root/package.json" ]] || fail "Run this from a complete Writing Environment checkout."
[[ -f "$repo_root/pnpm-lock.yaml" ]] || fail "pnpm-lock.yaml is missing."

if [[ "$skip_packages" == "0" ]]; then
  info "Installing Debian/Ubuntu and Tauri dependencies."
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
    util-linux \
    wget
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

info "Building the optimized x86-64 application."
(
  cd "$repo_root"
  pnpm install --frozen-lockfile
  pnpm tauri build --no-bundle
)

readonly built_binary="$repo_root/src-tauri/target/release/writing-environment"
[[ -x "$built_binary" ]] || fail "The release binary was not created at $built_binary"
file "$built_binary" | grep -Eq 'x86-64|x86_64' || fail "The build did not produce an x86-64 executable."

info "Installing the application and desktop launcher."
sudo install -Dm755 "$built_binary" /opt/writing-environment/writing-environment
sudo install -Dm755 "$repo_root/deploy/linux-amd64/writing-environment" /usr/local/bin/writing-environment
sudo install -Dm644 "$repo_root/deploy/linux-amd64/writing-environment.desktop" /usr/local/share/applications/writing-environment.desktop
sudo install -Dm644 "$repo_root/design/app-icon.svg" /usr/local/share/icons/hicolor/scalable/apps/writing-environment.svg

readonly config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/writing-environment"
mkdir -p "$config_dir"
if [[ ! -e "$config_dir/environment" ]]; then
  install -m 644 "$repo_root/deploy/linux-amd64/environment.example" "$config_dir/environment"
fi

if [[ "$autostart" == "1" ]]; then
  enable_xdg_autostart "$repo_root/deploy/linux-amd64/writing-environment.desktop"
  info "Enabled XDG desktop autostart."
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  sudo update-desktop-database /usr/local/share/applications >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  sudo gtk-update-icon-cache -q -t /usr/local/share/icons/hicolor >/dev/null 2>&1 || true
fi

info "Installation complete. Start it with: writing-environment"
