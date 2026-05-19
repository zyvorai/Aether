#!/usr/bin/env bash
# Quick API smoke checks against a running aether serve instance.
# Usage: AETHER_API_BASE=http://127.0.0.1:5090 ./scripts/smoke-api.sh
set -euo pipefail

BASE="${AETHER_API_BASE:-http://127.0.0.1:5090}"
AUTH=()
if [ -n "${AETHER_API_KEY:-}" ]; then
  AUTH=(-H "Authorization: Bearer ${AETHER_API_KEY}")
fi

check() {
  local path="$1"
  local code
  code="$(curl -sS -m 10 -o /dev/null -w '%{http_code}' "${AUTH[@]}" "${BASE}${path}")"
  if [ "${code}" = "200" ]; then
    echo "OK  ${path} (${code})"
  else
    echo "FAIL ${path} (${code})" >&2
    return 1
  fi
}

check /health
check /api/server
check /api/system/ready
check /api/auth/providers
check /api/gitops/status
echo "smoke-api: all checks passed"
