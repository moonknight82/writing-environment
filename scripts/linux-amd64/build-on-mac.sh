#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly script_dir

usage() {
  cat <<'EOF'
Usage: scripts/linux-amd64/build-on-mac.sh

Build the current Writing Environment source as a Linux AMD64 Debian package
from macOS. Docker Desktop performs the x86-64 build; no Linux toolchain needs
to be installed on the Mac. Output and SHA-256 checksums are written under
artifacts/linux-amd64/.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi
[[ $# -eq 0 ]] || { usage >&2; exit 2; }
[[ "$(uname -s)" == "Darwin" ]] || {
  printf 'This entry point is for macOS. Run scripts/linux-amd64/build-packages.sh on Linux.\n' >&2
  exit 1
}
command -v docker >/dev/null 2>&1 || {
  printf 'Docker Desktop is required.\n' >&2
  exit 1
}
docker info >/dev/null 2>&1 || {
  printf 'Docker is installed but its engine is not running. Start Docker Desktop and try again.\n' >&2
  exit 1
}

exec "$script_dir/build-packages.sh"
