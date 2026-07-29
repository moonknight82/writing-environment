#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
readonly builder_tag="writing-environment-apt-builder:trixie"

app_deb=""
output=""
private_key="${WRITING_ENVIRONMENT_APT_SIGNING_KEY:-$repo_root/.apt-signing/writing-environment-apt-private.asc}"
public_key="$repo_root/deploy/apt-repository/writing-environment-archive-keyring.asc"

usage() {
  cat <<'EOF'
Usage: scripts/apt/build-on-mac.sh --app-deb FILE [options]

Build the appliance Debian packages and a signed static APT repository in an
ARM64 Debian Trixie container.

Options:
  --app-deb FILE       Tauri ARM64 Debian package (required)
  --output DIRECTORY   New output directory (default: timestamped artifact)
  --private-key FILE   Repository signing private key
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app-deb) app_deb="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    --private-key) private_key="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ "$(uname -s)" == "Darwin" ]] || { printf 'This entry point requires macOS.\n' >&2; exit 1; }
[[ -f "$app_deb" ]] || { printf 'Application Debian package not found: %s\n' "$app_deb" >&2; exit 1; }
[[ -f "$private_key" ]] || { printf 'APT signing private key not found: %s\n' "$private_key" >&2; exit 1; }
[[ -f "$public_key" ]] || { printf 'APT signing public key not found: %s\n' "$public_key" >&2; exit 1; }
command -v docker >/dev/null 2>&1 || { printf 'Docker Desktop is required.\n' >&2; exit 1; }
docker info >/dev/null 2>&1 || { printf 'Start Docker Desktop before building the repository.\n' >&2; exit 1; }

if [[ -z "$output" ]]; then
  output="$repo_root/artifacts/apt-repository/mac-$(date +%Y%m%d-%H%M%S)"
fi
[[ ! -e "$output" ]] || { printf 'Refusing to overwrite existing output: %s\n' "$output" >&2; exit 1; }

readonly temporary="$(mktemp -d /tmp/writing-environment-apt-build.XXXXXX)"
trap 'rm -rf -- "$temporary"' EXIT
plugin_output="$temporary/plugin"
"$repo_root/scripts/pi-image/build-settings-plugin.sh" --output "$plugin_output"
mkdir -p "$output"

docker build \
  --platform linux/arm64 \
  --tag "$builder_tag" \
  "$repo_root/deploy/apt-repository"

docker run --rm --platform linux/arm64 \
  --volume "$repo_root:/workspace:ro" \
  --volume "$(dirname "$app_deb"):/app:ro" \
  --volume "$plugin_output:/plugin:ro" \
  --volume "$output:/output" \
  "$builder_tag" \
  /workspace/scripts/apt/build-packages.sh \
    --app-deb "/app/$(basename "$app_deb")" \
    --settings-plugin /plugin/librpcc_writing-environment-bluetooth.so \
    --public-key /workspace/deploy/apt-repository/writing-environment-archive-keyring.asc \
    --output /output/packages

docker run --rm --platform linux/arm64 \
  --volume "$repo_root:/workspace:ro" \
  --volume "$private_key:/keys/private.asc:ro" \
  --volume "$output:/output" \
  "$builder_tag" \
  /workspace/scripts/apt/build-repository.sh \
    --packages /output/packages \
    --private-key /keys/private.asc \
    --output /output/repository

(
  cd "$output/repository"
  shasum -a 256 -c SHA256SUMS
)

printf 'Writing Environment APT release written to:\n%s\n' "$output"
