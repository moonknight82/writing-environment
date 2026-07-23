#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
readonly script_dir
readonly repo_root
launch_app=0

usage() {
  cat <<'EOF'
Usage: scripts/pi/benchmark.sh [--launch]

Capture a Pi 4 system/performance snapshot. If --launch is supplied and the
app is not already running, the script opens it and waits ten seconds before
recording its resident memory and CPU usage. The app is left open afterward.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --launch) launch_app=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
  shift
done

report_dir="$repo_root/reports/pi"
mkdir -p "$report_dir"
report="$report_dir/benchmark-$(date +%Y%m%d-%H%M%S).txt"
binary="/usr/bin/writing-environment"
[[ -x "$binary" ]] || binary="/opt/writing-environment/writing-environment"

app_pid="$(pgrep -n -f '^(/usr/bin|/opt/writing-environment)/writing-environment( |$)' || true)"
if [[ -z "$app_pid" && "$launch_app" == "1" ]]; then
  command -v writing-environment >/dev/null 2>&1 || { printf 'Writing Environment is not installed.\n' >&2; exit 1; }
  writing-environment >/dev/null 2>&1 &
  for _ in {1..40}; do
    app_pid="$(pgrep -n -f '^(/usr/bin|/opt/writing-environment)/writing-environment( |$)' || true)"
    [[ -n "$app_pid" ]] && break
    sleep 0.25
  done
  [[ -n "$app_pid" ]] || { printf 'The application did not start.\n' >&2; exit 1; }
  sleep 10
fi

{
  printf 'Writing Environment — Raspberry Pi benchmark\n'
  printf 'Captured: %s\n\n' "$(date --iso-8601=seconds)"

  printf 'SYSTEM\n'
  if [[ -r /proc/device-tree/model ]]; then
    printf 'Model: '; tr -d '\0' </proc/device-tree/model; printf '\n'
  fi
  printf 'OS: '
  if [[ -r /etc/os-release ]]; then
    # shellcheck source=/dev/null
    source /etc/os-release
    printf '%s\n' "${PRETTY_NAME:-unknown}"
  else
    printf 'unknown\n'
  fi
  printf 'Kernel: %s\n' "$(uname -sr)"
  printf 'Architecture: %s\n' "$(uname -m)"
  printf 'Uptime: %s\n\n' "$(uptime -p 2>/dev/null || true)"

  printf 'MEMORY\n'
  free -h
  printf '\nSTORAGE\n'
  df -h "$HOME" "$repo_root" | awk '!seen[$0]++'
  printf '\nTHERMAL / POWER\n'
  if command -v vcgencmd >/dev/null 2>&1; then
    vcgencmd measure_temp
    vcgencmd get_throttled
  else
    printf 'vcgencmd is unavailable.\n'
  fi

  printf '\nAPPLICATION\n'
  if [[ -x "$binary" ]]; then
    printf 'Binary: %s\n' "$binary"
    printf 'Binary size: %s\n' "$(du -h "$binary" | awk '{print $1}')"
  else
    printf 'Installed binary not found.\n'
  fi
  if [[ -n "$app_pid" ]] && kill -0 "$app_pid" 2>/dev/null; then
    ps -p "$app_pid" -o pid=,etimes=,rss=,%cpu=,command= | awk '{
      printf "PID: %s\nElapsed seconds: %s\nResident KiB: %s\nCPU: %s%%\nCommand:", $1, $2, $3, $4
      for (field = 5; field <= NF; field++) printf " %s", $field
      printf "\n"
    }'
    if [[ -r "/proc/$app_pid/environ" ]]; then
      renderer_override="$(tr '\0' '\n' <"/proc/$app_pid/environ" | sed -n 's/^WEBKIT_DISABLE_DMABUF_RENDERER=//p' | tail -n 1)"
      printf 'DMA-BUF renderer disabled: %s\n' "${renderer_override:-not set}"
    fi
  else
    printf 'App was not running. Re-run with --launch to capture its idle process.\n'
  fi

  cat <<'EOF'

MANUAL ACCEPTANCE PASS
[ ] Open a generated 1,000-sheet test project and time its first scan.
[ ] Open the 100,000-word manuscript and type continuously for one minute.
[ ] Switch every visual theme and change line height.
[ ] Toggle paragraph and sentence focus while typing.
[ ] Disconnect networking; edit, close, and reopen a sheet.
[ ] Confirm idle resident memory remains below the provisional 350 MB gate.
[ ] Confirm vcgencmd get_throttled reports 0x0 after the stress pass.

Generate a disposable project with:
  scripts/pi/generate-test-library.sh
EOF
} >"$report"

printf 'Benchmark report written to:\n%s\n' "$report"
