#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
readonly plugin_context="$repo_root/deploy/pi-image/settings-plugin"

output=""

usage() {
  cat <<'EOF'
Usage: scripts/pi-image/build-settings-plugin.sh --output DIRECTORY

Build the Writing Environment Raspberry Pi Control Centre Bluetooth plugin as
a Linux ARM64 shared library using Docker Desktop.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output) output="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ -n "$output" ]] || { printf -- '--output is required.\n' >&2; exit 2; }
if [[ -e "$output" && ! -d "$output" ]]; then
  printf 'The plugin output path is not a directory: %s\n' "$output" >&2
  exit 1
fi
if [[ -d "$output" ]] && find "$output" -mindepth 1 -print -quit | grep -q .; then
  printf 'Refusing to overwrite non-empty output: %s\n' "$output" >&2
  exit 1
fi
command -v docker >/dev/null 2>&1 || { printf 'Docker is required.\n' >&2; exit 1; }
docker info >/dev/null 2>&1 || { printf 'Start Docker before building the plugin.\n' >&2; exit 1; }

mkdir -p "$output"
docker build \
  --platform linux/arm64 \
  --target export \
  --output "type=local,dest=$output" \
  "$plugin_context"

plugin="$output/librpcc_writing-environment-bluetooth.so"
[[ -f "$plugin" ]] || { printf 'The plugin build produced no shared library.\n' >&2; exit 1; }
file "$plugin" | grep -qE 'ELF 64-bit.*(ARM aarch64|aarch64)' || {
  printf 'The built plugin is not Linux ARM64.\n' >&2
  exit 1
}
printf 'Built Raspberry Pi Control Centre plugin:\n%s\n' "$plugin"
