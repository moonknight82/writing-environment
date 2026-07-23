#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
readonly script_dir
readonly repo_root
# The file is copied beside this script in a prebuilt artifact.
# shellcheck disable=SC1091
source "$script_dir/lib.sh"

skip_packages=0
allow_unsupported=0

usage() {
  cat <<'EOF'
Usage: scripts/pi/build-on-pi.sh [options]

Build a self-contained Linux ARM64 artifact natively on a Raspberry Pi.

Options:
  --skip-packages      Do not install or update build dependencies
  --allow-unsupported  Permit a compatible non-Pi ARM64 Debian build host
  -h, --help           Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-packages) skip_packages=1 ;;
    --allow-unsupported) allow_unsupported=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; fail "Unknown option: $1" ;;
  esac
  shift
done

require_regular_user
check_supported_host "$allow_unsupported"

if [[ "$skip_packages" == "0" ]]; then
  info "Installing Raspberry Pi build dependencies."
  sudo apt-get update
  sudo apt-get install -y \
    build-essential \
    curl \
    file \
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
require_command file

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

info "Building the optimized native ARM64 application."
(
  cd "$repo_root"
  export CARGO_TARGET_DIR="$cargo_target_dir"
  pnpm install --frozen-lockfile
  pnpm tauri build --bundles deb --config '{"bundle":{"createUpdaterArtifacts":false}}'
)

timestamp="$(date +%Y%m%d-%H%M%S)"
output="$repo_root/artifacts/pi-arm64/pi-$timestamp"
"$script_dir/package-artifact.sh" "$output" "pi-$timestamp"

printf 'Pi artifact built natively:\n%s\n' "$output"
