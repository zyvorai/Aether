#!/usr/bin/env bash
# ============================================================================
# confidential-fabric-e2e.sh — Smoke Aether confidential APIs + optional Ragnarok composite
#
# Usage:
#   AETHER_API=http://212.8.252.194:30090 ./scripts/confidential-fabric-e2e.sh
#   RAGNAROK_API=http://212.8.252.194:30062 ./scripts/confidential-fabric-e2e.sh
#   REQUIRE_SNP=1 AETHER_API_KEY=... ./scripts/confidential-fabric-e2e.sh
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/aether-confidential-smoke.sh
source "${SCRIPT_DIR}/lib/aether-confidential-smoke.sh"

echo "=== Aether confidential fabric E2E ==="
aether_smoke_confidential_apis || exit 1
aether_check_tee_capabilities || exit 1
ragnarok_composite_check || true

if [ -n "${RAGNAROK_API:-}" ] && [ -f "${SCRIPT_DIR}/../ragnarok/scripts/confidential-fabric-e2e.sh" ]; then
  echo ""
  echo "── Delegating to Ragnarok fabric smoke ──"
  API_BASE="${RAGNAROK_API}" \
    bash "${SCRIPT_DIR}/../ragnarok/scripts/confidential-fabric-e2e.sh" || aether_warn "Ragnarok fabric smoke failed (optional)"
elif [ -n "${RAGNAROK_API:-}" ]; then
  RAGNAROK_ROOT="${RAGNAROK_REPO:-$(cd "${SCRIPT_DIR}/../../ragnarok" 2>/dev/null && pwd || true)}"
  if [ -n "${RAGNAROK_ROOT}" ] && [ -f "${RAGNAROK_ROOT}/scripts/confidential-fabric-e2e.sh" ]; then
    API_BASE="${RAGNAROK_API}" bash "${RAGNAROK_ROOT}/scripts/confidential-fabric-e2e.sh" || true
  fi
fi

echo "=== Aether confidential fabric E2E passed ==="
