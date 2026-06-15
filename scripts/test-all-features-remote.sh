#!/usr/bin/env bash
# ============================================================================
# test-all-features-remote.sh — orchestrate Aether E2E tiers from laptop
# ============================================================================
#
# Usage:
#   ./scripts/test-all-features-remote.sh <host> [ssh_user]
#
# Environment:
#   AETHER_TEST_TIERS   Comma list, or preset: quick | full
#     quick → smoke
#     full  → smoke,labs-live,confidential,deploy-remove,playwright-all
#   AETHER_API          Base URL (default http://HOST:30090)
#   AETHER_E2E_REPORT_JSON  Optional path for JSON summary
#   AETHER_E2E_KIND=1   Enables playwright-exec tier (kind cluster exec tests)
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
HOST="${1:-${AETHER_REMOTE_HOST:-${DEPLOY_HOST:-}}}"
USER="${2:-${AETHER_REMOTE_USER:-${DEPLOY_USER:-sus}}}"
NODE_PORT="${AETHER_NODE_PORT:-30090}"
TIERS_RAW="${AETHER_TEST_TIERS:-smoke,labs-dry,confidential}"

if [[ -z "${HOST}" ]]; then
    echo "Usage: $0 <host> [ssh_user]" >&2
    exit 1
fi

case "${TIERS_RAW}" in
    quick) TIERS="smoke" ;;
    full) TIERS="smoke,labs-live,confidential,deploy-remove,playwright-all" ;;
    *) TIERS="${TIERS_RAW}" ;;
esac

export AETHER_API="${AETHER_API:-http://${HOST}:${NODE_PORT}}"
export AETHER_REMOTE_HOST="${HOST}"
export AETHER_REMOTE_USER="${USER}"
export DEPLOY_HOST="${HOST}"
export DEPLOY_USER="${USER}"

REMOTE_DIR="${AETHER_REMOTE_DIR:-~/.deployment/aether}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=20 -o StrictHostKeyChecking=accept-new)

FAIL=0
PASS_TIERS=0
SUITE_START=$(date +%s)
TIER_RESULTS=()

B='\033[1m'
G='\033[0;32m'
R='\033[0;31m'
N='\033[0m'

tier_enabled() {
    echo "${TIERS}" | tr ',' '\n' | grep -qx "${1}"
}

record_tier() {
    local name="$1" status="$2" elapsed="$3"
    TIER_RESULTS+=("${name}:${status}:${elapsed}")
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
    {
        echo "{"
        echo "  \"product\": \"aether\","
        echo "  \"host\": \"${HOST}\","
        echo "  \"api\": \"${AETHER_API}\","
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

echo -e "${B}Aether feature test suite${N}"
echo "  API:    ${AETHER_API}"
echo "  SSH:    ${USER}@${HOST}"
echo "  Tiers:  ${TIERS}"
echo ""

if tier_enabled smoke; then
    run_tier_cmd smoke env AETHER_API="${AETHER_API}" "${SCRIPT_DIR}/post-deploy-verify.sh" || true
fi

if tier_enabled labs-dry; then
    run_tier_cmd labs-dry bash -c "
        set -euo pipefail
        cd '${ROOT}'
        ensure_local() {
          if [[ -x ./target/release/aether ]]; then export AETHER_BIN=./target/release/aether; return; fi
          cargo build --release
          export AETHER_BIN=./target/release/aether
        }
        ensure_local
        chmod +x scripts/reference-cluster-e2e.sh scripts/labs-e2e.sh scripts/k8s-labs-e2e.sh
        AETHER_BIN=\"\${AETHER_BIN}\" scripts/reference-cluster-e2e.sh
    " || true
fi

if tier_enabled labs-live; then
    run_tier_cmd labs-live bash -c "
        set -euo pipefail
        SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=20 -o StrictHostKeyChecking=accept-new)
        echo '  Syncing lab scripts to ${USER}@${HOST}:${REMOTE_DIR}/scripts/'
        ssh \"\${SSH_OPTS[@]}\" '${USER}@${HOST}' \"mkdir -p ${REMOTE_DIR}/scripts\"
        rsync -az -e \"ssh \${SSH_OPTS[*]}\" \
            '${ROOT}/scripts/k8s-labs-e2e.sh' \
            '${ROOT}/scripts/labs-e2e.sh' \
            '${USER}@${HOST}:${REMOTE_DIR}/scripts/'
        ssh \"\${SSH_OPTS[@]}\" '${USER}@${HOST}' \
            \"cd ${REMOTE_DIR} && chmod +x scripts/k8s-labs-e2e.sh scripts/labs-e2e.sh && \
             AETHER_LABS_LIVE=1 AETHER_BIN=./target/release/aether ./scripts/k8s-labs-e2e.sh\"
    " || true
fi

if tier_enabled confidential; then
    run_tier_cmd confidential bash -c "
        set -euo pipefail
        cd '${ROOT}'
        chmod +x scripts/confidential-fabric-e2e.sh scripts/confidential-cluster-e2e.sh \
            scripts/lib/aether-confidential-smoke.sh
        AETHER_API='${AETHER_API}' AETHER_TEE_SNP=1 scripts/confidential-fabric-e2e.sh
        if [[ -x ./target/release/aether ]]; then AETHER_BIN=./target/release/aether; \
          elif [[ -x ./target/debug/aether ]]; then AETHER_BIN=./target/debug/aether; \
          else cargo build --release && AETHER_BIN=./target/release/aether; fi
        AETHER_API='${AETHER_API}' AETHER_TEE_SNP=1 AETHER_BIN=\"\${AETHER_BIN}\" \
            scripts/confidential-cluster-e2e.sh
    " || true
fi

if tier_enabled deploy-remove; then
    run_tier_cmd deploy-remove bash -c "
        set -euo pipefail
        cd '${ROOT}'
        chmod +x scripts/deploy-remove-e2e.sh
        if [[ -x ./target/release/aether ]]; then AETHER_BIN=./target/release/aether; \
          elif [[ -x ./target/debug/aether ]]; then AETHER_BIN=./target/debug/aether; \
          else cargo build --release && AETHER_BIN=./target/release/aether; fi
        AETHER_API='${AETHER_API}' AETHER_BIN=\"\${AETHER_BIN}\" scripts/deploy-remove-e2e.sh
    " || true
fi

if tier_enabled playwright-all; then
    _pw_timeout="${AETHER_PLAYWRIGHT_TIMEOUT:-}"
    run_tier_cmd playwright-all bash -c "
        set -euo pipefail
        export AETHER_E2E_SKIP_SERVER=1
        export AETHER_E2E_BASE_URL='${AETHER_API}'
        cd '${ROOT}/web/dashboard'
        npm ci --silent
        npx playwright install chromium
        if [[ -n '${_pw_timeout}' ]]; then
            export PLAYWRIGHT_TIMEOUT='${_pw_timeout}'
        fi
        npm run test:e2e
    " || true
fi

if tier_enabled playwright-exec; then
    if [[ "${AETHER_E2E_KIND:-0}" == "1" ]]; then
        run_tier_cmd playwright-exec bash -c "
            set -euo pipefail
            cd '${ROOT}'
            chmod +x scripts/kind-playwright-fixture.sh
            export AETHER_E2E_SKIP_SERVER=1
            export AETHER_E2E_BASE_URL='${AETHER_API}'
            export AETHER_E2E_KIND=1
            scripts/kind-playwright-fixture.sh
            cd web/dashboard
            npm run test:e2e -- tests/cluster-exec-terminal.spec.ts
        " || true
    else
        echo ""
        echo -e "${B}  Tier: playwright-exec (skipped — set AETHER_E2E_KIND=1)${N}"
        record_tier playwright-exec skipped 0
    fi
fi

write_report_json

echo ""
echo -e "${B}══════════════════════════════════════════════${N}"
if [[ "${FAIL}" -eq 0 ]]; then
    echo -e "${G}${B}  ALL TIERS PASSED${N}"
    echo "  See docs/TEST_PLAN.md for manual-only scenarios."
    exit 0
fi
echo -e "${R}${B}  ${FAIL} tier(s) failed — see docs/TEST_PLAN.md${N}"
exit 1
