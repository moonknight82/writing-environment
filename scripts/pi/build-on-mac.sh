#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
readonly script_dir
readonly repo_root

usage() {
  cat <<'EOF'
Usage: scripts/pi/build-on-mac.sh

Use Docker Desktop on macOS to build a self-contained Linux ARM64 artifact for
the Raspberry Pi. Output is written to a timestamped artifacts/pi-arm64 folder.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi
[[ $# -eq 0 ]] || { usage >&2; exit 2; }
[[ "$(uname -s)" == "Darwin" ]] || { printf 'This script must run on macOS. Use build-on-pi.sh on the Pi.\n' >&2; exit 1; }
command -v docker >/dev/null 2>&1 || { printf 'Docker Desktop is required.\n' >&2; exit 1; }
docker info >/dev/null 2>&1 || { printf 'Docker is installed but its engine is not running. Start Docker Desktop and try again.\n' >&2; exit 1; }

host_arch="$(uname -m)"
if [[ "$host_arch" != "arm64" ]]; then
  printf 'Warning: %s macOS will emulate ARM64 and may build slowly.\n' "$host_arch" >&2
fi

timestamp="$(date +%Y%m%d-%H%M%S)"
output="$repo_root/artifacts/pi-arm64/mac-$timestamp"
[[ ! -e "$output" ]] || { printf 'Refusing to overwrite %s\n' "$output" >&2; exit 1; }
mkdir -p "$output"

docker_args=(docker build)
signing_key="${WRITING_ENVIRONMENT_SIGNING_KEY:-$HOME/.tauri/writing-environment.key}"
if [[ -s "$signing_key" ]]; then
  docker_args+=(--secret "id=tauri_signing_private_key,src=$signing_key")
fi
"${docker_args[@]}" \
  --platform linux/arm64 \
  --build-arg "BUILD_ID=mac-$timestamp" \
  --file "$repo_root/deploy/pi/Dockerfile" \
  --target export \
  --output "type=local,dest=$output" \
  "$repo_root"

archive="$(find "$output" -maxdepth 1 -type f -name '*.tar.gz' -print -quit)"
checksum="$(find "$output" -maxdepth 1 -type f -name '*.tar.gz.sha256' -print -quit)"
[[ -n "$archive" && -n "$checksum" ]] || { printf 'Expected artifact and checksum were not exported.\n' >&2; exit 1; }
(
  cd "$output"
  shasum -a 256 -c "$(basename "$checksum")"
)

printf 'Pi artifact built on macOS:\n%s\nCopy it to the Pi, extract it, and run ./install.sh.\n' "$archive"
