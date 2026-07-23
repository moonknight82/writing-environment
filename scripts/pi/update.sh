#!/usr/bin/env bash
set -euo pipefail

readonly script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly repo_root="$(cd "$script_dir/../.." && pwd)"
pull_first=0

usage() {
  cat <<'EOF'
Usage: scripts/pi/update.sh [--pull]

Rebuild and reinstall the current checkout without changing the existing
autostart setting. Use --pull to fast-forward a clean Git checkout first.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pull) pull_first=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
  shift
done

if [[ "$pull_first" == "1" ]]; then
  command -v git >/dev/null 2>&1 || { printf 'git is required for --pull\n' >&2; exit 1; }
  [[ -d "$repo_root/.git" ]] || { printf 'This checkout has no Git metadata.\n' >&2; exit 1; }
  if ! git -C "$repo_root" diff --quiet || ! git -C "$repo_root" diff --cached --quiet; then
    printf 'Refusing to pull over local tracked changes. Commit or stash them first.\n' >&2
    exit 1
  fi
  git -C "$repo_root" pull --ff-only
fi

exec "$script_dir/install.sh" --skip-packages --preserve-autostart
