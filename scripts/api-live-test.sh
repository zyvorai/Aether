#!/usr/bin/env bash
# ============================================================================
# api-live-test.sh — live E2E orchestrator (API walkthrough + live tiers)
# ============================================================================
#
# Default tiers (AETHER_LIVE_TIERS=all):
#   api-live, smoke, deploy-remove, confidential, labs-live, playwright-all
#
# Usage:
#   ./scripts/api-live-test.sh
#   ./scripts/api-live-test.sh 212.8.252.194 sus
#   AETHER_LIVE_TIERS=quick ./scripts/api-live-test.sh
#   AETHER_API=http://HOST:30090 ./scripts/api-live-test.sh
#
# api-live tier options (forwarded to api-live-runner.py):
#   --mutations     Also probe PUT/PATCH/DELETE
#   --methods GET   Restrict HTTP methods (comma-separated)
#   --limit N       Stop after N API probes
#
# Environment:
#   AETHER_LIVE_TIERS            Preset all|quick or comma tier list
#   AETHER_API / AETHER_API_BASE Base URL
#   AETHER_REMOTE_HOST           Required for labs-live SSH tier
#   AETHER_REMOTE_USER           SSH user (default sus)
#   AETHER_API_KEY               Bearer token (or mock IdP session)
#   AETHER_API_LIVE_LOG          JSONL capture for api-live tier
#   AETHER_E2E_REPORT_JSON       Tier timing JSON summary
#   AETHER_PLAYWRIGHT_TIMEOUT    Optional Playwright timeout override
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUNNER="${ROOT}/scripts/lib/api-live-runner.py"

NODE_PORT="${AETHER_NODE_PORT:-30090}"
TIERS_RAW="${AETHER_LIVE_TIERS:-all}"

API_LIVE_ARGS=()
POSITIONAL=()
for arg in "$@"; do
  if [[ "${arg}" == --* ]]; then
    API_LIVE_ARGS+=("${arg}")
  else
    POSITIONAL+=("${arg}")
  fi
done

HOST="${POSITIONAL[0]:-${AETHER_REMOTE_HOST:-${DEPLOY_HOST:-}}}"
USER="${POSITIONAL[1]:-${AETHER_REMOTE_USER:-${DEPLOY_USER:-sus}}}"

case "${TIERS_RAW}" in
  all) TIERS="api-live,smoke,deploy-remove,confidential,labs-live,playwright-all" ;;
  quick) TIERS="api-live,smoke" ;;
  *) TIERS="${TIERS_RAW}" ;;
esac

if [[ -n "${HOST}" ]]; then
  export AETHER_API="${AETHER_API:-http://${HOST}:${NODE_PORT}}"
  export AETHER_REMOTE_HOST="${HOST}"
  export AETHER_REMOTE_USER="${USER}"
  export DEPLOY_HOST="${HOST}"
  export DEPLOY_USER="${USER}"
else
  export AETHER_API="${AETHER_API:-${AETHER_API_BASE:-http://127.0.0.1:5090}}"
fi

API="${AETHER_API}"
REMOTE_DIR="${AETHER_REMOTE_DIR:-~/.deployment/aether}"
SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=20 -o StrictHostKeyChecking=accept-new)

# shellcheck source=lib/e2e-tier-runner.sh
source "${SCRIPT_DIR}/lib/e2e-tier-runner.sh"
e2e_tier_init

# shellcheck source=lib/post-deploy-auth.sh
source "${SCRIPT_DIR}/lib/post-deploy-auth.sh"

echo -e "${B}Aether live E2E suite${N}"
echo "  API:    ${API}"
if [[ -n "${HOST}" ]]; then
  echo "  SSH:    ${USER}@${HOST}"
fi
echo "  Tiers:  ${TIERS}"
echo ""

if tier_enabled api-live; then
  start=$(date +%s)
  echo ""
  echo -e "${B}══════════════════════════════════════════════${N}"
  echo -e "${B}  Tier: api-live${N}"
  echo -e "${B}══════════════════════════════════════════════${N}"
  set +e
  rc=0
  if ! post_deploy_auth_prepare "${API}"; then
    echo "ERROR: auth bootstrap failed (set AETHER_API_KEY or run server with AETHER_MOCK_IDP=1)" >&2
    rc=1
  else
    AUTH_HEADER=""
    if [ "${#AUTH[@]}" -gt 0 ]; then
      AUTH_HEADER="${AUTH[1]}"
    fi
    COOKIE_HEADER=""
    if [ -n "${AETHER_POST_DEPLOY_COOKIE_JAR:-}" ] && [ -s "${AETHER_POST_DEPLOY_COOKIE_JAR}" ]; then
      COOKIE_HEADER="$(post_deploy_auth_cookie_header || true)"
    fi
    export AETHER_API_LIVE_AUTH_HEADER="${AUTH_HEADER}"
    export AETHER_API_LIVE_COOKIE="${COOKIE_HEADER}"
    RUNNER_ARGS=(--base "${API}")
    if [ -n "${AETHER_API_LIVE_LOG:-}" ]; then
      RUNNER_ARGS+=(--log "${AETHER_API_LIVE_LOG}")
    fi
    if [ "${#API_LIVE_ARGS[@]}" -gt 0 ]; then
      RUNNER_ARGS+=("${API_LIVE_ARGS[@]}")
    fi
    python3 "${RUNNER}" "${RUNNER_ARGS[@]}"
    rc=$?
  fi
  set -e
  end=$(date +%s)
  elapsed=$((end - start))
  if [[ "${rc}" -eq 0 ]]; then
    echo -e "${G}  Tier api-live: PASSED (${elapsed}s)${N}"
    PASS_TIERS=$((PASS_TIERS + 1))
    record_tier api-live passed "${elapsed}"
  else
    echo -e "${R}  Tier api-live: FAILED (${elapsed}s)${N}"
    FAIL=$((FAIL + 1))
    record_tier api-live failed "${elapsed}"
  fi
fi

if tier_enabled smoke; then
  run_tier_cmd smoke env AETHER_API="${API}" "${SCRIPT_DIR}/post-deploy-verify.sh" || true
fi

if tier_enabled deploy-remove; then
  run_tier_cmd deploy-remove bash -c "
    set -euo pipefail
    cd '${ROOT}'
    chmod +x scripts/deploy-remove-e2e.sh
    if [[ -x ./target/release/aether ]]; then AETHER_BIN=./target/release/aether; \
      elif [[ -x ./target/debug/aether ]]; then AETHER_BIN=./target/debug/aether; \
      else cargo build --release && AETHER_BIN=./target/release/aether; fi
    AETHER_API='${API}' AETHER_BIN=\"\${AETHER_BIN}\" scripts/deploy-remove-e2e.sh
  " || true
fi

if tier_enabled confidential; then
  run_tier_cmd confidential bash -c "
    set -euo pipefail
    cd '${ROOT}'
    chmod +x scripts/confidential-fabric-e2e.sh scripts/confidential-cluster-e2e.sh \
      scripts/lib/aether-confidential-smoke.sh
    AETHER_API='${API}' AETHER_TEE_SNP=1 scripts/confidential-fabric-e2e.sh
    if [[ -x ./target/release/aether ]]; then AETHER_BIN=./target/release/aether; \
      elif [[ -x ./target/debug/aether ]]; then AETHER_BIN=./target/debug/aether; \
      else cargo build --release && AETHER_BIN=./target/release/aether; fi
    AETHER_API='${API}' AETHER_TEE_SNP=1 AETHER_BIN=\"\${AETHER_BIN}\" \
      scripts/confidential-cluster-e2e.sh
  " || true
fi

if tier_enabled labs-live; then
  if [[ -z "${AETHER_REMOTE_HOST:-}" ]]; then
    record_tier_skipped labs-live "AETHER_REMOTE_HOST unset"
  else
    run_tier_cmd labs-live bash -c "
      set -euo pipefail
      SSH_OPTS=(-o BatchMode=yes -o ConnectTimeout=20 -o StrictHostKeyChecking=accept-new)
      echo '  Syncing lab scripts to ${USER}@${AETHER_REMOTE_HOST}:${REMOTE_DIR}/scripts/'
      ssh \"\${SSH_OPTS[@]}\" '${USER}@${AETHER_REMOTE_HOST}' \"mkdir -p ${REMOTE_DIR}/scripts\"
      rsync -az -e \"ssh \${SSH_OPTS[*]}\" \
        '${ROOT}/scripts/k8s-labs-e2e.sh' \
        '${ROOT}/scripts/labs-e2e.sh' \
        '${USER}@${AETHER_REMOTE_HOST}:${REMOTE_DIR}/scripts/'
      ssh \"\${SSH_OPTS[@]}\" '${USER}@${AETHER_REMOTE_HOST}' \
        \"cd ${REMOTE_DIR} && chmod +x scripts/k8s-labs-e2e.sh scripts/labs-e2e.sh && \
         AETHER_LABS_LIVE=1 AETHER_BIN=./target/release/aether ./scripts/k8s-labs-e2e.sh\"
    " || true
  fi
fi

if tier_enabled playwright-all; then
  _pw_timeout="${AETHER_PLAYWRIGHT_TIMEOUT:-}"
  run_tier_cmd playwright-all bash -c "
    set -euo pipefail
    export AETHER_E2E_SKIP_SERVER=1
    export AETHER_E2E_BASE_URL='${API}'
    cd '${ROOT}/web/dashboard'
    npm ci --silent
    npx playwright install chromium
    if [[ -n '${_pw_timeout}' ]]; then
      export PLAYWRIGHT_TIMEOUT='${_pw_timeout}'
    fi
    npm run test:e2e
  " || true
fi

write_report_json

if print_suite_summary; then
  exit 0
fi
exit 1
