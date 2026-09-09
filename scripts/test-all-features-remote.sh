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
#     full  → smoke,labs-live,deploy-remove,playwright-all
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
TIERS_RAW="${AETHER_TEST_TIERS:-smoke,labs-dry}"

if [[ -z "${HOST}" ]]; then
    echo "Usage: $0 <host> [ssh_user]" >&2
    exit 1
fi

case "${TIERS_RAW}" in
    quick) TIERS="smoke" ;;
    full) TIERS="smoke,labs-live,deploy-remove,playwright-all" ;;
    *) TIERS="${TIERS_RAW}" ;;
esac

export AETHER_API="${AETHER_API:-http://${HOST}:${NODE_PORT}}"
export AETHER_REMOTE_HOST="${HOST}"
export AETHER_REMOTE_USER="${USER}"
export DEPLOY_HOST="${HOST}"
export DEPLOY_USER="${USER}"

REMOTE_DIR="${AETHER_REMOTE_DIR:-~/.deployment/aether}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=20 -o StrictHostKeyChecking=accept-new)

# shellcheck source=lib/e2e-tier-runner.sh
source "${SCRIPT_DIR}/lib/e2e-tier-runner.sh"
e2e_tier_init

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
    record_tier_skipped confidential "Ragnarok confidential E2E is not shipped in this repository"
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
        record_tier_skipped playwright-exec "set AETHER_E2E_KIND=1"
    fi
fi

write_report_json

if print_suite_summary; then
    exit 0
fi
exit 1
