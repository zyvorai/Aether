#!/usr/bin/env bash
# ============================================================================
# test-platform-remote.sh — Aether + Hermes remote E2E from laptop
# ============================================================================
#
# Usage:
#   ./scripts/test-platform-remote.sh <host> [ssh_user]
#   AETHER_TEST_TIERS=full HERMES_TEST_TIERS=full ./scripts/test-platform-remote.sh 212.8.252.194 sus
#
# Environment:
#   AETHER_TEST_TIERS   quick | full | comma list (default: full)
#   HERMES_TEST_TIERS   quick | full | comma list (default: full)
#   AETHER_API          default http://HOST:30090
#   HERMES_E2E_BASE     default http://HOST:31847
#   PLATFORM_E2E_REPORT_JSON  optional combined JSON summary
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
AETHER_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=lib/resolve-zyvor-sibling.sh
source "${SCRIPT_DIR}/lib/resolve-zyvor-sibling.sh"

HOST="${1:-${DEPLOY_HOST:-${AETHER_REMOTE_HOST:-}}}"
USER="${2:-${DEPLOY_USER:-${AETHER_REMOTE_USER:-sus}}}"
AETHER_PORT="${AETHER_NODE_PORT:-30090}"
HERMES_PORT="${HERMES_NODE_PORT:-31847}"

AETHER_TIERS="${AETHER_TEST_TIERS:-full}"
HERMES_TIERS="${HERMES_TEST_TIERS:-full}"

if [[ -z "${HOST}" ]]; then
  echo "Usage: $0 <host> [ssh_user]" >&2
  exit 1
fi

HERMES_ROOT="$(resolve_zyvor_sibling "${AETHER_ROOT}" hermes)" || {
  echo "ERROR: Hermes repo not found — clone sibling hermes/ or set HERMES_REPO." >&2
  exit 1
}

export DEPLOY_HOST="${HOST}"
export DEPLOY_USER="${USER}"
export AETHER_API="${AETHER_API:-http://${HOST}:${AETHER_PORT}}"
export HERMES_E2E_BASE="${HERMES_E2E_BASE:-http://${HOST}:${HERMES_PORT}}"

FAIL=0
PASS_PRODUCTS=0
SUITE_START=$(date +%s)
PRODUCT_RESULTS=()

B='\033[1m'
G='\033[0;32m'
R='\033[0;31m'
Y='\033[0;33m'
N='\033[0m'

record_product() {
  local name="$1" status="$2" elapsed="$3"
  PRODUCT_RESULTS+=("${name}:${status}:${elapsed}")
}

preflight() {
  echo -e "${B}Preflight${N}"
  echo "  Aether: ${AETHER_API}"
  echo "  Hermes: ${HERMES_E2E_BASE}"
  echo "  SSH:    ${USER}@${HOST}"
  local ok=0
  if curl -sf -m 15 "${AETHER_API}/health" >/dev/null; then
    echo -e "  ${G}✓${N} Aether /health"
  else
    echo -e "  ${R}✗${N} Aether /health unreachable" >&2
    ok=1
  fi
  if curl -sf -m 15 "${HERMES_E2E_BASE}/healthz" >/dev/null; then
    echo -e "  ${G}✓${N} Hermes /healthz"
  else
    echo -e "  ${R}✗${N} Hermes /healthz unreachable" >&2
    ok=1
  fi
  if ssh -o BatchMode=yes -o ConnectTimeout=15 "${USER}@${HOST}" echo ok >/dev/null 2>&1; then
    echo -e "  ${G}✓${N} SSH"
  else
    echo -e "  ${Y}!${N} SSH check skipped or failed (API tests may still run)"
  fi
  [[ "${ok}" -eq 0 ]] || return 1
}

run_product() {
  local name="$1" script="$2" tiers_env="$3" tiers_val="$4"
  local start end elapsed rc
  start=$(date +%s)
  echo ""
  echo -e "${B}══════════════════════════════════════════════${N}"
  echo -e "${B}  Product: ${name}  (${tiers_val})${N}"
  echo -e "${B}══════════════════════════════════════════════${N}"
  set +e
  env "${tiers_env}=${tiers_val}" "${script}" "${HOST}" "${USER}"
  rc=$?
  set -e
  end=$(date +%s)
  elapsed=$((end - start))
  if [[ "${rc}" -eq 0 ]]; then
    echo -e "${G}  ${name}: PASSED (${elapsed}s)${N}"
    PASS_PRODUCTS=$((PASS_PRODUCTS + 1))
    record_product "${name}" "passed" "${elapsed}"
    return 0
  fi
  echo -e "${R}  ${name}: FAILED (${elapsed}s)${N}"
  FAIL=$((FAIL + 1))
  record_product "${name}" "failed" "${elapsed}"
  return 1
}

write_report_json() {
  local report="${PLATFORM_E2E_REPORT_JSON:-}"
  [[ -z "${report}" ]] && return 0
  local total_elapsed=$(( $(date +%s) - SUITE_START ))
  {
    echo "{"
    echo "  \"host\": \"${HOST}\","
    echo "  \"aether_api\": \"${AETHER_API}\","
    echo "  \"hermes_api\": \"${HERMES_E2E_BASE}\","
    echo "  \"aether_tiers\": \"${AETHER_TIERS}\","
    echo "  \"hermes_tiers\": \"${HERMES_TIERS}\","
    echo "  \"passed_products\": ${PASS_PRODUCTS},"
    echo "  \"failed_products\": ${FAIL},"
    echo "  \"elapsed_seconds\": ${total_elapsed},"
    echo "  \"results\": ["
    local i=0
    for entry in "${PRODUCT_RESULTS[@]}"; do
      IFS=':' read -r name status elapsed <<< "${entry}"
      [[ $i -gt 0 ]] && echo ","
      printf '    {"product":"%s","status":"%s","elapsed_seconds":%s}' "${name}" "${status}" "${elapsed}"
      i=$((i + 1))
    done
    echo ""
    echo "  ]"
    echo "}"
  } > "${report}"
  echo "  Report: ${report}"
}

echo -e "${B}Platform remote test (Aether + Hermes)${N}"
preflight || exit 1

chmod +x "${SCRIPT_DIR}/test-all-features-remote.sh" \
  "${HERMES_ROOT}/scripts/test-all-features-remote.sh" 2>/dev/null || true

run_product "Aether" "${SCRIPT_DIR}/test-all-features-remote.sh" "AETHER_TEST_TIERS" "${AETHER_TIERS}" || true
run_product "Hermes" "${HERMES_ROOT}/scripts/test-all-features-remote.sh" "HERMES_TEST_TIERS" "${HERMES_TIERS}" || true

write_report_json

echo ""
echo -e "${B}══════════════════════════════════════════════${N}"
if [[ "${FAIL}" -eq 0 ]]; then
  echo -e "${G}${B}  PLATFORM: ALL PRODUCTS PASSED${N}"
  exit 0
fi
echo -e "${R}${B}  PLATFORM: ${FAIL} product(s) failed${N}"
exit 1
