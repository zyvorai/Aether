#!/usr/bin/env bash
# Deploy Aether with mock IdP SSO enabled (see examples/deploy/reference-sso.env.example).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${AETHER_DEPLOY_ENV:-$ROOT/examples/deploy/reference-sso.env.example}"
if [ -f "${ENV_FILE}" ]; then
  set -a
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  set +a
fi
export AETHER_REFERENCE_MOCK_IDP="${AETHER_REFERENCE_MOCK_IDP:-1}"
export AETHER_MOCK_IDP=1
export AETHER_SESSION_SECRET="${AETHER_SESSION_SECRET:-mock-idp-dev-session-key-32chars}"
exec "${ROOT}/scripts/deploy-remote.sh" "$@"
