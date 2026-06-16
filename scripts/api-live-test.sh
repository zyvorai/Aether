#!/usr/bin/env bash
# ============================================================================
# api-live-test.sh — live terminal walkthrough of Aether REST APIs
# ============================================================================
#
# Exercises routes discovered from src/api/mod.rs against a running server.
# Prints each request/response to the terminal as it runs; optionally captures
# full bodies to a JSON-lines log file.
#
# Usage:
#   ./scripts/api-live-test.sh
#   AETHER_API=http://127.0.0.1:5090 ./scripts/api-live-test.sh
#   AETHER_API=http://HOST:30090 AETHER_API_KEY=... ./scripts/api-live-test.sh
#   AETHER_API_LIVE_LOG=/tmp/aether-api-live.jsonl ./scripts/api-live-test.sh
#
# Options (forwarded to runner):
#   --mutations     Also probe PUT/PATCH/DELETE (default: GET + POST only)
#   --methods GET   Restrict HTTP methods (comma-separated)
#   --limit N       Stop after N probes (0 = all; useful for quick smoke)
#
# Environment:
#   AETHER_API / AETHER_API_BASE   Base URL (default http://127.0.0.1:5090)
#   AETHER_API_KEY                 Bearer token (or use mock IdP session bootstrap)
#   AETHER_API_LIVE_LOG            JSONL capture path for full responses
#   AETHER_API_LIVE_BODY_MAX       Max chars shown per response in terminal
#   AETHER_API_LIVE_TIMEOUT        Per-request timeout seconds (default 20)
#   AETHER_API_LIVE_METHODS        Comma methods to probe (default GET,POST)
# ============================================================================

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API="${AETHER_API:-${AETHER_API_BASE:-http://127.0.0.1:5090}}"
RUNNER="${ROOT}/scripts/lib/api-live-runner.py"

# shellcheck source=lib/post-deploy-auth.sh
source "${ROOT}/scripts/lib/post-deploy-auth.sh"

post_deploy_auth_prepare "${API}" || {
  echo "ERROR: auth bootstrap failed (set AETHER_API_KEY or run server with AETHER_MOCK_IDP=1)" >&2
  exit 1
}

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

ARGS=(--base "${API}")
if [ -n "${AETHER_API_LIVE_LOG:-}" ]; then
  ARGS+=(--log "${AETHER_API_LIVE_LOG}")
fi

exec python3 "${RUNNER}" "${ARGS[@]}" "$@"
