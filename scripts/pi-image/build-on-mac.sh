#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
readonly image_gen_commit="d409bd28212648941f16e1f3f8cad5f863f17ce8"
readonly image_gen_tag="writing-environment-rpi-image-gen:${image_gen_commit:0:12}"

artifact=""
password_hash="${WRITING_ENVIRONMENT_IMAGE_PASSWORD_HASH:-}"
use_default_password=0
image_locale="${WRITING_ENVIRONMENT_IMAGE_LOCALE:-en_US.UTF-8}"
keyboard_keymap="${WRITING_ENVIRONMENT_IMAGE_KEYBOARD_KEYMAP:-us}"
keyboard_layout="${WRITING_ENVIRONMENT_IMAGE_KEYBOARD_LAYOUT:-English (US)}"
image_timezone="${WRITING_ENVIRONMENT_IMAGE_TIMEZONE:-America/Sao_Paulo}"
wifi_country="${WRITING_ENVIRONMENT_IMAGE_WIFI_COUNTRY:-BR}"

usage() {
  cat <<'EOF'
Usage: scripts/pi-image/build-on-mac.sh [options]

Build a personalized Raspberry Pi 4 image on an Apple Silicon Mac using
Docker Desktop. If no artifact is supplied, the newest Pi ARM64 artifact under
artifacts/pi-arm64 is used. If no password option is supplied, the script
prompts for the recovery-user password without storing the plaintext value.

Options:
  --artifact FILE             Pi ARM64 application artifact
  --password-hash HASH        SHA-512 crypt recovery password hash
  --default-password          Set the writer password to "writer"
  --locale LOCALE             System locale (default: en_US.UTF-8)
  --keyboard-keymap KEYMAP    Console/XKB keymap (default: us)
  --keyboard-layout LABEL     Debian keyboard variant label (default: English (US))
  --timezone ZONE             IANA timezone (default: America/Sao_Paulo)
  --wifi-country CODE         Two-letter regulatory country (default: BR)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifact) artifact="${2:-}"; shift 2 ;;
    --password-hash) password_hash="${2:-}"; shift 2 ;;
    --default-password) use_default_password=1; shift ;;
    --locale) image_locale="${2:-}"; shift 2 ;;
    --keyboard-keymap) keyboard_keymap="${2:-}"; shift 2 ;;
    --keyboard-layout) keyboard_layout="${2:-}"; shift 2 ;;
    --timezone) image_timezone="${2:-}"; shift 2 ;;
    --wifi-country) wifi_country="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ "$(uname -s)" == "Darwin" ]] || { printf 'This entry point requires macOS.\n' >&2; exit 1; }
[[ "$(uname -m)" == "arm64" ]] || { printf 'An Apple Silicon Mac is required.\n' >&2; exit 1; }
command -v docker >/dev/null 2>&1 || { printf 'Docker Desktop is required.\n' >&2; exit 1; }
docker info >/dev/null 2>&1 || { printf 'Start Docker Desktop before building the image.\n' >&2; exit 1; }

if [[ -z "$artifact" ]]; then
  artifact="$(find "$repo_root/artifacts/pi-arm64" -type f -name '*.tar.gz' -print 2>/dev/null | LC_ALL=C sort | tail -n 1)"
fi
[[ -f "$artifact" ]] || { printf 'No Pi ARM64 application artifact was found.\n' >&2; exit 1; }

if [[ "$use_default_password" == "1" && -n "$password_hash" ]]; then
  printf 'Use either --default-password or --password-hash, not both.\n' >&2
  exit 2
fi

if [[ "$use_default_password" == "1" ]]; then
  password_hash="$(printf '%s' 'writer' | openssl passwd -6 -stdin)"
elif [[ -z "$password_hash" ]]; then
  printf 'Choose a password for the writer recovery account.\n'
  IFS= read -r -s -p 'Password: ' password
  printf '\n'
  IFS= read -r -s -p 'Confirm password: ' confirmation
  printf '\n'
  [[ "$password" == "$confirmation" ]] || { printf 'Passwords did not match.\n' >&2; exit 1; }
  [[ "${#password}" -ge 10 ]] || { printf 'Use at least 10 characters.\n' >&2; exit 1; }
  password_hash="$(printf '%s' "$password" | openssl passwd -6 -stdin)"
  unset password confirmation
fi
[[ "$password_hash" == \$6\$* ]] || { printf 'Expected a SHA-512 crypt password hash beginning with $6$.\n' >&2; exit 2; }
[[ "$image_locale" =~ ^[A-Za-z]{2}_[A-Za-z]{2}\.UTF-8$ ]] || { printf 'Invalid locale: %s\n' "$image_locale" >&2; exit 2; }
[[ "$keyboard_keymap" =~ ^[A-Za-z0-9_+.-]+$ ]] || { printf 'Invalid keyboard keymap: %s\n' "$keyboard_keymap" >&2; exit 2; }
[[ -n "$keyboard_layout" && "$keyboard_layout" != *$'\n'* ]] || { printf 'Invalid keyboard layout label.\n' >&2; exit 2; }
[[ "$image_timezone" =~ ^[A-Za-z0-9_+.-]+(/[A-Za-z0-9_+.-]+)+$ ]] || { printf 'Invalid timezone: %s\n' "$image_timezone" >&2; exit 2; }
[[ "$wifi_country" =~ ^[A-Z]{2}$ ]] || { printf 'Invalid Wi-Fi country code: %s\n' "$wifi_country" >&2; exit 2; }

timestamp="$(date +%Y%m%d-%H%M%S)"
output="$repo_root/artifacts/pi-image/mac-$timestamp"
source_dir="$output/source"
mkdir -p "$output"
"$script_dir/prepare-source.sh" --artifact "$artifact" --output "$source_dir"

docker build \
  --platform linux/arm64 \
  --build-arg "RPI_IMAGE_GEN_COMMIT=$image_gen_commit" \
  --tag "$image_gen_tag" \
  "$repo_root/deploy/pi-image"

docker run --rm \
  --platform linux/arm64 \
  --privileged \
  --security-opt seccomp=unconfined \
  --env "WRITER_PASSWORD_HASH=$password_hash" \
  --env "IMAGE_LOCALE=$image_locale" \
  --env "IMAGE_KEYBOARD_KEYMAP=$keyboard_keymap" \
  --env "IMAGE_KEYBOARD_LAYOUT=$keyboard_layout" \
  --env "IMAGE_TIMEZONE=$image_timezone" \
  --env "IMAGE_WIFI_COUNTRY=$wifi_country" \
  --volume "$source_dir:/source:ro" \
  --volume "$output:/output" \
  "$image_gen_tag"

(
  cd "$output"
  shasum -a 256 -c SHA256SUMS
)
printf 'Writing Environment Raspberry Pi image written to:\n%s\n' "$output"
