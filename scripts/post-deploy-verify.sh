#!/usr/bin/env bash
# Post-deploy verification: health, API smoke, CloudOS endpoints.
set -euo pipefail

API="${AETHER_API:-${1:-http://127.0.0.1:5090}}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# shellcheck source=lib/post-deploy-auth.sh
source "${ROOT}/scripts/lib/post-deploy-auth.sh"
post_deploy_auth_prepare "${API}" || {
  echo "ERROR: post-deploy auth bootstrap failed (set AETHER_API_KEY or AETHER_MOCK_IDP=1 on server)" >&2
  exit 1
}
export AETHER_POST_DEPLOY_COOKIE_JAR
export AETHER_POST_DEPLOY_COOKIE_HEADER
AETHER_POST_DEPLOY_COOKIE_HEADER="$(post_deploy_auth_cookie_header || true)"
trap 'rm -f "${AETHER_POST_DEPLOY_COOKIE_JAR:-}"' EXIT

echo "==> Post-deploy verify → ${API}"
curl -sf "${API}/health" >/dev/null
echo "  ok: /health"

AETHER_API="${API}" "${ROOT}/scripts/remote-api-ux-verify.sh"
AETHER_API="${API}" "${ROOT}/scripts/k8s-api-smoke.sh"
echo "==> Post-deploy verification passed"
