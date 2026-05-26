#!/usr/bin/env bash
# ============================================================================
# confidential-fabric-e2e.sh — Smoke composite + SPIRE + trust APIs on a live lab
#
# Usage:
#   API_BASE=http://212.8.252.194:30062 ./scripts/confidential-fabric-e2e.sh
#   SPIRE_EXPECT_AGENT=1 ./scripts/confidential-fabric-e2e.sh
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/remote-api-smoke.sh
source "${SCRIPT_DIR}/lib/remote-api-smoke.sh"

API_BASE="${API_BASE:-http://127.0.0.1:30062}"
export API_BASE UI_BASE="${UI_BASE:-http://127.0.0.1:30061}"

echo "=== Confidential fabric E2E ==="
run_remote_api_smoke || exit 1

token=$(curl -sf -X POST "${API_BASE}/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"${ADMIN_USER:-admin}\",\"password\":\"${ADMIN_PASS:-Admin@321}\"}" \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")

composite=$(curl -sf "${API_BASE}/api/v1/confidential/composite/status")
echo "${composite}" | python3 -c "
import json,sys
d=json.load(sys.stdin).get('data',{})
assert d.get('aether_url_configured'), 'aether_url_configured'
print('  OK: composite aether_url_configured')
if d.get('aether_reachable'):
    print('  OK: composite aether_reachable')
else:
    print('  WARN: aether not reachable (optional)')
"

spire=$(curl -sf -H "Authorization: Bearer ${token}" "${API_BASE}/api/v1/confidential/spire/status")
echo "${spire}" | python3 -c "
import json,sys,os
d=json.load(sys.stdin).get('data',{})
print('  SPIRE enabled:', d.get('enabled'))
print('  SPIRE agent_reachable:', d.get('agent_reachable'))
if os.environ.get('SPIRE_EXPECT_AGENT') == '1':
    assert d.get('enabled'), 'SPIRE should be enabled'
    assert d.get('agent_reachable'), 'SPIRE agent socket should exist'
    print('  OK: SPIRE agent reachable (SPIRE_EXPECT_AGENT=1)')
"

if [ "${TRUST_ENFORCE_EXPECT:-}" = "1" ]; then
  code=$(curl -s -o /tmp/trust-enforce.json -w '%{http_code}' \
    -X POST "${API_BASE}/api/v1/confidential/trust/evaluate" \
    -H "Authorization: Bearer ${token}" \
    -H 'Content-Type: application/json' \
    -d '{"vm_id":"e2e-smoke","trust_composite":0.1,"attestation_passed":false,"debug_allowed":true}')
  if [ "$code" = "200" ]; then
    echo "  OK: POST /confidential/trust/evaluate (HTTP $code)"
  else
    echo "  FAIL: trust/evaluate HTTP $code" >&2
    exit 1
  fi
fi

echo "=== Confidential fabric E2E passed ==="
