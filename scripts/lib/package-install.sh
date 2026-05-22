#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Aether client install                                     ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "► Step 1/4 — Dependencies…"
[ -x ./install-client-deps.sh ] && ./install-client-deps.sh || true
echo ""
echo "► Step 2/4 — Configuration…"
[ -f aether.env.example ] && [ ! -f aether.env ] && cp aether.env.example aether.env && echo "  Created aether.env"
echo ""
echo "► Step 3/4 — Verify…"
test -x ./aether || { echo "ERROR: ./aether missing"; exit 1; }
echo "  OK: aether binary (dashboard embedded)"
echo ""
echo "► Step 4/4 — Test…"
[ -x ./test-package.sh ] && ./test-package.sh || true
echo ""
echo "  Remove: ./uninstall.sh --yes  (add --remove-dir to delete this folder)"
echo "══════════════════════════════════════════════════════════"
echo "  ./aether serve --host 0.0.0.0 --port 5090"
echo "  https://<host>:5090/web/dashboard/"
echo "  cat README.txt"
echo "  Remove: ./uninstall.sh --yes  (add --remove-dir to delete this folder)"
echo "══════════════════════════════════════════════════════════"
