#!/usr/bin/env bash
# Post-deploy verification: health, API smoke, CloudOS endpoints.
set -euo pipefail

API="${AETHER_API:-${1:-http://127.0.0.1:5090}}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Post-deploy verify → ${API}"
curl -sf "${API}/health" >/dev/null
echo "  ok: /health"

AETHER_API="${API}" "${ROOT}/scripts/remote-api-ux-verify.sh"
AETHER_API="${API}" "${ROOT}/scripts/k8s-api-smoke.sh"
echo "==> Post-deploy verification passed"
