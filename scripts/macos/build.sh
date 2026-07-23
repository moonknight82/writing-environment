#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
readonly script_dir repo_root

[[ "$(uname -s)" == "Darwin" ]] || { printf 'This script must run on macOS.\n' >&2; exit 1; }
[[ "$(uname -m)" == "arm64" ]] || { printf 'The current macOS release target is Apple Silicon.\n' >&2; exit 1; }

signing_key="${WRITING_ENVIRONMENT_SIGNING_KEY:-$HOME/.tauri/writing-environment.key}"
[[ -s "$signing_key" ]] || {
  printf 'Updater signing key not found at %s. Set WRITING_ENVIRONMENT_SIGNING_KEY to its path.\n' "$signing_key" >&2
  exit 1
}

version="$(sed -n 's/^[[:space:]]*"version": "\([^"]*\)".*/\1/p' "$repo_root/src-tauri/tauri.conf.json" | head -n 1)"
[[ -n "$version" ]] || { printf 'Could not read the application version.\n' >&2; exit 1; }
timestamp="$(date +%Y%m%d-%H%M%S)"
output="$repo_root/artifacts/macos/mac-$timestamp"
mkdir -p "$output"

(
  cd "$repo_root"
  export TAURI_SIGNING_PRIVATE_KEY="$(<"$signing_key")"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
  export APPLE_SIGNING_IDENTITY="-"
  pnpm install --frozen-lockfile
  pnpm tauri build --target aarch64-apple-darwin --bundles app
)

bundle_dir="$repo_root/src-tauri/target/aarch64-apple-darwin/release/bundle/macos"
app="$bundle_dir/Writing Environment.app"
updater="$bundle_dir/Writing Environment.app.tar.gz"
[[ -d "$app" && -f "$updater" && -f "$updater.sig" ]] || {
  printf 'Expected macOS application and signed updater artifacts were not created.\n' >&2
  exit 1
}

ditto -c -k --sequesterRsrc --keepParent "$app" "$output/Writing-Environment-macOS-$version.zip"
cp "$updater" "$updater.sig" "$output/"
(
  cd "$output"
  shasum -a 256 * >SHA256SUMS
)

printf 'Signed macOS artifacts written to:\n%s\n' "$output"
