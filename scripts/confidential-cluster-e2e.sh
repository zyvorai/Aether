#!/usr/bin/env bash
# ============================================================================
# confidential-cluster-e2e.sh — Deploy, placement, and migration on SNP-capable clusters
#
# Validates confidential posture end-to-end against a live Aether control plane
# (and optional Ragnarok composite). Live cluster steps are opt-in.
#
# Usage:
#   AETHER_API=http://lab:30090 AETHER_API_KEY=secret ./scripts/confidential-cluster-e2e.sh
#   REQUIRE_SNP=1 ./scripts/confidential-cluster-e2e.sh
#   AETHER_CONFIDENTIAL_LIVE=1 KUBECONFIG=~/.kube/config ./scripts/confidential-cluster-e2e.sh
#
# Environment:
#   AETHER_BIN            aether binary (default ./target/release/aether)
#   SPEC                  workload YAML (default examples/confidential-snp.yaml)
#   MIGRATE_SPEC          migration example (default examples/confidential-migrate-kubevirt.yaml)
#   AETHER_CONFIDENTIAL_LIVE=1   run deploy + placement API + migration plan API
#   AETHER_CONFIDENTIAL_MIGRATE=1  run confidential-blue-green migrate (destructive)
#   AETHER_TEE_SNP=1      advertise SNP in dev/lab when /dev/sev absent
#   RAGNAROK_API          optional composite hub URL
# ============================================================================

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/aether-confidential-smoke.sh
source "${SCRIPT_DIR}/lib/aether-confidential-smoke.sh"

AETHER="${AETHER_BIN:-./target/release/aether}"
SPEC="${SPEC:-examples/confidential-snp.yaml}"
MIGRATE_SPEC="${MIGRATE_SPEC:-examples/confidential-migrate-kubevirt.yaml}"
LIVE="${AETHER_CONFIDENTIAL_LIVE:-0}"
MIGRATE_LIVE="${AETHER_CONFIDENTIAL_MIGRATE:-0}"

if [ ! -x "${AETHER}" ] && [ -x "./target/debug/aether" ]; then
  AETHER="./target/debug/aether"
fi
[ -x "${AETHER}" ] || { echo "Missing aether binary; run: cargo build --release" >&2; exit 1; }
[ -f "${SPEC}" ] || { echo "Missing spec: ${SPEC}" >&2; exit 1; }

yaml_workload_name() {
  awk '/^metadata:/{m=1} m && /^  name:/{print $2; exit}' "$1"
}
WORKLOAD="$(yaml_workload_name "${SPEC}")"
WORKLOAD="${WORKLOAD:-confidential-app}"

aether_cli() {
  if "${AETHER}" confidential --help 2>&1 | grep -q 'placement'; then
    "${AETHER}" "$@"
  else
    echo "  (using cargo run — rebuild release binary for faster runs)" >&2
    cargo run --quiet --release -- "$@"
  fi
}

MIGRATE_WORKLOAD="$(yaml_workload_name "${MIGRATE_SPEC}")"
MIGRATE_WORKLOAD="${MIGRATE_WORKLOAD:-confidential-migrate-demo}"

echo "=== Confidential cluster E2E ==="
echo "  spec=${SPEC} workload=${WORKLOAD}"
echo "  migrate_spec=${MIGRATE_SPEC}"
echo "  live=${LIVE} migrate_live=${MIGRATE_LIVE}"
echo ""

# ── API smoke + TEE inventory ──
aether_smoke_confidential_apis || exit 1
aether_check_tee_capabilities || exit 1
ragnarok_composite_check || true

# ── CLI: validate + placement (spec file, no cluster required) ──
echo ""
echo "── CLI placement & policy ──"
aether_cli --spec "${SPEC}" validate
aether_ok "validate ${SPEC}"
aether_cli --spec "${SPEC}" confidential placement
aether_ok "confidential placement ${WORKLOAD}"
aether_cli --spec "${SPEC}" confidential isolation-check || aether_warn "isolation-check issues"
aether_cli --spec "${MIGRATE_SPEC}" confidential migration plan --target kubevirt
aether_ok "migration plan ${MIGRATE_WORKLOAD}"

# ── Ragnarok trust placement (optional) ──
if [ -n "${RAGNAROK_API:-}" ]; then
  echo ""
  echo "── Ragnarok confidential placement ──"
  token="$(curl -sf -X POST "${RAGNAROK_API%/}/api/v1/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"${ADMIN_USER:-admin}\",\"password\":\"${ADMIN_PASS:-Admin@321}\"}" \
    | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")"
  code=$(curl -s -o /tmp/rgn-placement.json -w '%{http_code}' \
    -X POST "${RAGNAROK_API%/}/api/v1/intelligence/placement/confidential" \
    -H "Authorization: Bearer ${token}" \
    -H 'Content-Type: application/json' \
    -d "{\"vm_id\":\"${WORKLOAD}\",\"confidential\":{\"enabled\":true,\"tee\":\"sev-snp\",\"attestation\":{\"required\":true,\"policy\":\"standard\"},\"isolation\":{\"vtpm\":true,\"encryptedState\":true,\"debugAllowed\":false}}}")
  if [ "${code}" = "200" ]; then
    aether_ok "POST /intelligence/placement/confidential"
    python3 -c "
import json
d=json.load(open('/tmp/rgn-placement.json')).get('data',{})
rec=d.get('recommendation') or d
print('  top node:', (rec.get('candidates') or [{}])[0].get('node','—'))
"
  else
    aether_warn "Ragnarok placement HTTP ${code}"
  fi
fi

if [ "${LIVE}" != "1" ]; then
  echo ""
  echo "  (skip live cluster: set AETHER_CONFIDENTIAL_LIVE=1)"
  echo "=== Confidential cluster E2E (API + CLI) passed ==="
  exit 0
fi

if [ -z "${KUBECONFIG:-}" ] && [ ! -f "${HOME}/.kube/config" ]; then
  echo "FAIL: AETHER_CONFIDENTIAL_LIVE=1 requires kubeconfig" >&2
  exit 1
fi

echo ""
echo "── Live cluster: deploy + API placement + migration plan ──"
export AETHER_TEE_SNP="${AETHER_TEE_SNP:-1}"
export AETHER_MIGRATION_TARGET_SNP="${AETHER_MIGRATION_TARGET_SNP:-1}"

aether_cli run --spec "${SPEC}" --runtime kubevirt --dry-run
aether_ok "kubevirt dry-run ${WORKLOAD}"

aether_cli run --spec "${SPEC}" --runtime kubevirt
aether_ok "deploy ${WORKLOAD} to kubevirt"

sleep 2
code="$(aether_fetch_json "/api/confidential/placement/${WORKLOAD}" /tmp/aether-placement.json)"
if [ "${code}" = "200" ]; then
  aether_ok "GET /api/confidential/placement/${WORKLOAD}"
  python3 -c "
import json
d=json.load(open('/tmp/aether-placement.json')).get('data',{})
print('  recommended_runtime:', d.get('recommended_runtime'))
print('  host_tee_ready:', d.get('host_tee_ready'))
print('  blockers:', d.get('blockers'))
"
else
  aether_warn "placement API HTTP ${code}"
fi

code="$(aether_fetch_json "/api/confidential/migration-plan/${WORKLOAD}/kubevirt" /tmp/aether-migplan.json)"
if [ "${code}" = "200" ]; then
  aether_ok "GET /api/confidential/migration-plan/${WORKLOAD}/kubevirt"
  python3 -c "
import json
d=json.load(open('/tmp/aether-migplan.json')).get('data',{})
print('  strategy:', d.get('recommended_strategy'))
print('  uri:', (d.get('encrypted_migration_uri') or '')[:60], '...')
"
else
  aether_warn "migration-plan API HTTP ${code}"
fi

if [ "${MIGRATE_LIVE}" = "1" ]; then
  echo ""
  echo "── Live confidential-blue-green migrate ──"
  aether_cli migrate "${WORKLOAD}" --target kubevirt --strategy confidential-blue-green \
    || aether_fail "migrate ${WORKLOAD}"
  aether_ok "confidential-blue-green migrate ${WORKLOAD}"
fi

echo ""
echo "=== Confidential cluster E2E (live) passed ==="
