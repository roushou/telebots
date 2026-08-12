#!/usr/bin/env bash
# Runs the Rust JSON API (port 9110, in-container only) and the TanStack
# Start dashboard server (port 3000, host-facing) side by side.
set -uo pipefail

monitor &
monitor_pid=$!

node /app/web/server/index.mjs &
web_pid=$!

shutdown() {
  kill "$monitor_pid" "$web_pid" 2>/dev/null || true
  wait "$monitor_pid" "$web_pid" 2>/dev/null || true
}
trap shutdown INT TERM

# Exit when either process exits, then bring down the other.
wait -n "$monitor_pid" "$web_pid"
code=$?
shutdown
exit "$code"
