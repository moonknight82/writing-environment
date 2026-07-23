#!/usr/bin/env bash
set -euo pipefail

sheet_count=1000
base_dir="${HOME}/Writing Environment Benchmarks"

usage() {
  cat <<'EOF'
Usage: scripts/pi/generate-test-library.sh [--sheets NUMBER] [--base DIR]

Create a new, timestamped disposable Markdown project for Pi performance tests.
The project includes many small sheets and one roughly 100,000-word manuscript.
No existing directory is overwritten.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --sheets)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      sheet_count="$2"
      shift
      ;;
    --base)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      base_dir="$2"
      shift
      ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
  shift
done

[[ "$sheet_count" =~ ^[1-9][0-9]*$ ]] || { printf 'Sheet count must be a positive integer.\n' >&2; exit 2; }

timestamp="$(date +%Y%m%d-%H%M%S)"
target="$base_dir/Pi Test Library $timestamp"
[[ ! -e "$target" ]] || { printf 'Refusing to overwrite %s\n' "$target" >&2; exit 1; }
mkdir -p "$target/Draft" "$target/Notes"

for ((index = 1; index <= sheet_count; index += 1)); do
  printf -- '---\ntitle: Test Sheet %04d\n---\n\n# Test Sheet %04d\n\nA small disposable sheet for measuring project scanning and navigation.\n' \
    "$index" "$index" >"$target/Notes/test-sheet-$(printf '%04d' "$index").md"
done

{
  printf -- '---\ntitle: One Hundred Thousand Words\n---\n\n# One Hundred Thousand Words\n\n'
  awk 'BEGIN {
    for (paragraph = 1; paragraph <= 1000; paragraph++) {
      for (word = 1; word <= 100; word++) {
        printf "word%s", (word == 100 ? "." : " ")
      }
      printf "\n\n"
    }
  }'
} >"$target/Draft/one-hundred-thousand-words.md"

printf 'Created disposable test project:\n%s\n' "$target"
printf 'Open this folder from Writing Environment, then delete it normally when testing is complete.\n'
