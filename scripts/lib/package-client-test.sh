#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
PORT="${AETHER_PORT:-5090}"
echo "== Aether package test =="
test -x ./aether && ./aether --help >/dev/null 2>&1 && echo "  OK: aether"
if curl -sf "http://127.0.0.1:${PORT}/health" >/dev/null 2>&1; then
  echo "  OK: health :${PORT}"
else
  echo "  SKIP: ./aether serve --host 0.0.0.0 --port ${PORT}"
fi
echo "Done."
