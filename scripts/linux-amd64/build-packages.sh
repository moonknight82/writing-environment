#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"

with_appimage=0

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  cat <<'EOF'
Usage: scripts/linux-amd64/build-packages.sh [--with-appimage]

Use Docker BuildKit and the Debian 12 amd64 baseline to create a .deb package.
On a native x86-64 Docker host, --with-appimage also creates an AppImage.
Output includes SHA-256 checksums in a new timestamped artifact directory.
EOF
  exit 0
fi
if [[ "${1:-}" == "--with-appimage" ]]; then
  with_appimage=1
  shift
fi
[[ $# -eq 0 ]] || { printf 'Unknown argument. Use --help for usage.\n' >&2; exit 2; }
command -v docker >/dev/null 2>&1 || { printf 'Docker is required.\n' >&2; exit 1; }
docker info >/dev/null 2>&1 || { printf 'Docker is installed but its engine is not running.\n' >&2; exit 1; }

bundles="deb"
if [[ "$with_appimage" == "1" ]]; then
  docker_arch="$(docker info --format '{{.Architecture}}')"
  if [[ "$docker_arch" != "x86_64" && "$docker_arch" != "amd64" ]]; then
    printf 'AppImage wrapping requires a native x86-64 Docker host; detected %s. Build the .deb here or run --with-appimage on an amd64 machine.\n' "$docker_arch" >&2
    exit 1
  fi
  bundles="appimage,deb"
fi

timestamp="$(date +%Y%m%d-%H%M%S)"
output="$repo_root/artifacts/linux-amd64/$timestamp"
[[ ! -e "$output" ]] || { printf 'Refusing to overwrite %s\n' "$output" >&2; exit 1; }
mkdir -p "$output"

docker_args=(docker build)
signing_key="${WRITING_ENVIRONMENT_SIGNING_KEY:-$HOME/.tauri/writing-environment.key}"
if [[ -s "$signing_key" ]]; then
  docker_args+=(--secret "id=tauri_signing_private_key,src=$signing_key")
fi
"${docker_args[@]}" \
  --platform linux/amd64 \
  --build-arg "TAURI_BUNDLES=$bundles" \
  --file "$repo_root/deploy/linux-amd64/Dockerfile" \
  --target export \
  --output "type=local,dest=$output" \
  "$repo_root"

deb_count="$(find "$output" -maxdepth 1 -type f -name '*.deb' -print | wc -l | tr -d ' ')"
[[ "$deb_count" -eq 1 ]] || {
  printf 'Expected exactly one current .deb artifact; found %s.\n' "$deb_count" >&2
  exit 1
}
if [[ "$with_appimage" == "1" ]]; then
  appimage_count="$(find "$output" -maxdepth 1 -type f -name '*.AppImage' -print | wc -l | tr -d ' ')"
  [[ "$appimage_count" -eq 1 ]] || {
    printf 'Expected exactly one current AppImage artifact; found %s.\n' "$appimage_count" >&2
    exit 1
  }
  appimage="$(find "$output" -maxdepth 1 -type f -name '*.AppImage' -print -quit)"
  file "$appimage" | grep -Eq 'x86-64|x86_64' || { printf 'AppImage is not x86-64.\n' >&2; exit 1; }
fi

printf 'Linux amd64 packages written to:\n%s\n' "$output"
cat "$output/SHA256SUMS"
