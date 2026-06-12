#!/usr/bin/env bash
# Deploy Aether with Active Directory / LDAP auth (see examples/deploy/ldap.env.example).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${AETHER_DEPLOY_ENV:-}"
if [ -z "${ENV_FILE}" ] || [ ! -f "${ENV_FILE}" ]; then
  echo "Set AETHER_DEPLOY_ENV to a private env file (see examples/deploy/ldap.env.example)" >&2
  exit 1
fi
set -a
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +a
if [ -z "${AETHER_LDAP_URL:-}" ] || [ -z "${AETHER_LDAP_BASE_DN:-}" ] || [ -z "${AETHER_SESSION_SECRET:-}" ]; then
  echo "AETHER_LDAP_URL, AETHER_LDAP_BASE_DN, and AETHER_SESSION_SECRET are required" >&2
  exit 1
fi
exec "${ROOT}/scripts/deploy-remote.sh" "$@"
