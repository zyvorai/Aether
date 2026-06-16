#!/usr/bin/env bash
# Shared tier runner for Aether E2E orchestrators.
#
# Callers must set before sourcing (or before calling init):
#   ROOT, TIERS, HOST (optional, for reports), AETHER_API (optional, for reports)
#
# Optional report env: AETHER_E2E_REPORT_JSON

# shellcheck disable=SC2034
e2e_tier_init() {
  FAIL=0
  PASS_TIERS=0
  SUITE_START=$(date +%s)
  TIER_RESULTS=()
}

B='\033[1m'
G='\033[0;32m'
R='\033[0;31m'
Y='\033[0;33m'
N='\033[0m'

tier_enabled() {
  echo "${TIERS}" | tr ',' '\n' | grep -qx "${1}"
}

record_tier() {
  local name="$1" status="$2" elapsed="$3"
  TIER_RESULTS+=("${name}:${status}:${elapsed}")
}

record_tier_skipped() {
  local name="$1" reason="${2:-skipped}"
  echo ""
  echo -e "${B}  Tier: ${name} (${reason})${N}"
  record_tier "${name}" "skipped" 0
}

run_tier_cmd() {
  local name="$1"
  shift
  local start end elapsed rc
  start=$(date +%s)
  echo ""
  echo -e "${B}══════════════════════════════════════════════${N}"
  echo -e "${B}  Tier: ${name}${N}"
  echo -e "${B}══════════════════════════════════════════════${N}"
  set +e
  "$@"
  rc=$?
  set -e
  end=$(date +%s)
  elapsed=$((end - start))
  if [[ "${rc}" -eq 0 ]]; then
    echo -e "${G}  Tier ${name}: PASSED (${elapsed}s)${N}"
    PASS_TIERS=$((PASS_TIERS + 1))
    record_tier "${name}" "passed" "${elapsed}"
    return 0
  fi
  echo -e "${R}  Tier ${name}: FAILED (${elapsed}s)${N}"
  FAIL=$((FAIL + 1))
  record_tier "${name}" "failed" "${elapsed}"
  return 1
}

write_report_json() {
  local report="${AETHER_E2E_REPORT_JSON:-}"
  [[ -z "${report}" ]] && return 0
  local total_elapsed=$(( $(date +%s) - SUITE_START ))
  local report_host="${HOST:-local}"
  local report_api="${AETHER_API:-}"
  {
    echo "{"
    echo "  \"product\": \"aether\","
    echo "  \"host\": \"${report_host}\","
    echo "  \"api\": \"${report_api}\","
    echo "  \"tiers\": \"${TIERS}\","
    echo "  \"passed_tiers\": ${PASS_TIERS},"
    echo "  \"failed_tiers\": ${FAIL},"
    echo "  \"elapsed_seconds\": ${total_elapsed},"
    echo "  \"results\": ["
    local i=0
    for entry in "${TIER_RESULTS[@]}"; do
      IFS=':' read -r name status elapsed <<< "${entry}"
      [[ $i -gt 0 ]] && echo ","
      printf '    {"tier":"%s","status":"%s","elapsed_seconds":%s}' "${name}" "${status}" "${elapsed}"
      i=$((i + 1))
    done
    echo ""
    echo "  ]"
    echo "}"
  } > "${report}"
  echo "  Report: ${report}"
}

ensure_local_aether() {
  local bin="${AETHER_BIN:-${ROOT}/target/release/aether}"
  if [[ -x "${bin}" ]]; then
    export AETHER_BIN="${bin}"
    return 0
  fi
  if [[ -x "${ROOT}/target/debug/aether" ]]; then
    export AETHER_BIN="${ROOT}/target/debug/aether"
    return 0
  fi
  echo "  Building local aether binary for CLI tiers..."
  (cd "${ROOT}" && cargo build --release)
  export AETHER_BIN="${ROOT}/target/release/aether"
}

print_suite_summary() {
  echo ""
  echo -e "${B}══════════════════════════════════════════════${N}"
  if [[ "${FAIL}" -eq 0 ]]; then
    echo -e "${G}${B}  ALL TIERS PASSED${N}"
    echo "  See docs/TEST_PLAN.md for manual-only scenarios."
    return 0
  fi
  echo -e "${R}${B}  ${FAIL} tier(s) failed — see docs/TEST_PLAN.md${N}"
  return 1
}
