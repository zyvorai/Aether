#!/usr/bin/env bash
# ==============================================================================
# test-license.sh — Zeus OS license API endpoint test harness
# ==============================================================================
#
# Tests the /api/license/* endpoints against a running Aether server.
# Starts and stops the server automatically; requires the aether binary.
#
# Usage:
#   ./scripts/test-license.sh [--help]
#
# Environment:
#   BASE_URL          Base URL of the server (default: http://localhost:5090)
#   AETHER_API_KEY    Bearer token for authenticated endpoints (optional)
#   AETHER_BIN        Path to the aether binary (auto-detected if unset)
#   LICENSEGEN_BIN    Path to the licensegen binary (auto-detected if unset)
#   TEST_PORT         Port to bind the test server (default: 15090)
#
# Exit codes:
#   0  All tests passed
#   1  One or more tests failed
# ==============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------- #
# Help
# ---------------------------------------------------------------------------- #
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'HELP'
test-license.sh — Zeus OS license API endpoint test harness

Tests the /api/license/* endpoints against a running Aether server.
Starts and stops the server automatically; requires the aether binary.

Usage:
  ./scripts/test-license.sh [--help]

Environment:
  BASE_URL          Base URL of the server (default: http://localhost:15090)
  AETHER_API_KEY    Bearer token for authenticated endpoints (optional)
  AETHER_BIN        Path to the aether binary (auto-detected if unset)
  LICENSEGEN_BIN    Path to the licensegen binary (auto-detected if unset)
  TEST_PORT         Port to bind the test server (default: 15090)

Suites:
  1. Valid license     — state VALID, customer present, reload endpoint
  2. Missing license   — state MISSING when no license path is configured
  3. Expired license   — state EXPIRED_GRACE for past valid_until date

Exit codes:
  0  All tests passed
  1  One or more tests failed
HELP
  exit 0
fi

# ---------------------------------------------------------------------------- #
# Config
# ---------------------------------------------------------------------------- #
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BASE_URL="${BASE_URL:-}"          # will be derived from TEST_PORT if empty
TEST_PORT="${TEST_PORT:-15090}"
AETHER_API_KEY="${AETHER_API_KEY:-}"

TEST_LICENSE="/tmp/test-zeus-valid.zyvor"
EXPIRED_LICENSE="/tmp/test-zeus-expired.zyvor"
SERVER_LOG="/tmp/aether-license-test-$$.log"
SERVER_PID=""

PASS=0
FAIL=0

# Colours (disabled when not a TTY)
if [[ -t 1 ]]; then
  GREEN="\033[0;32m"; RED="\033[0;31m"; YELLOW="\033[0;33m"; RESET="\033[0m"
else
  GREEN=""; RED=""; YELLOW=""; RESET=""
fi

# ---------------------------------------------------------------------------- #
# Helpers
# ---------------------------------------------------------------------------- #
info()  { printf "  %b\n" "$*"; }
warn()  { printf "  ${YELLOW}WARN${RESET}  %s\n" "$*"; }
pass()  { printf "  ${GREEN}PASS${RESET}  %s\n" "$*"; (( PASS++ )) || true; }
fail()  { printf "  ${RED}FAIL${RESET}  %s\n" "$*"; (( FAIL++ )) || true; }
die()   { printf "${RED}ERROR${RESET}: %s\n" "$*" >&2; exit 1; }
sep()   { printf "\n-- %s\n" "$*"; }

# JSON extraction: use jq if available, otherwise Python 3, otherwise grep.
jq_get() {
  local json="$1" key="$2"
  if command -v jq &>/dev/null; then
    printf '%s' "$json" | jq -r "$key" 2>/dev/null
  elif command -v python3 &>/dev/null; then
    printf '%s' "$json" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    keys = '$key'.lstrip('.').split('.')
    v = d
    for k in keys:
        v = v[k]
    print('' if v is None else v)
except Exception:
    sys.exit(1)
" 2>/dev/null
  else
    # Crude grep fallback for simple keys only
    local k
    k="$(printf '%s' "$key" | sed 's/^[.]\+//')"
    printf '%s' "$json" | grep -o "\"${k}\":[^,}]*" | head -1 | sed 's/.*: *"\{0,1\}\([^",}]*\)"\{0,1\}/\1/'
  fi
}

# Curl wrapper — returns body; sets CURL_STATUS to HTTP status code.
CURL_STATUS=""
do_curl() {
  local method="$1" url="$2"
  shift 2
  local extra_args=("$@")
  local headers=()
  [[ -n "${AETHER_API_KEY:-}" ]] && headers+=(-H "Authorization: Bearer ${AETHER_API_KEY}")
  CURL_STATUS=""
  local out
  out="$(curl -s -o /dev/stdout -w '\nHTTP_STATUS:%{http_code}' \
    -X "$method" "${extra_args[@]+"${extra_args[@]}"}" "${headers[@]+"${headers[@]}"}" "$url" 2>/dev/null)"
  CURL_STATUS="${out##*$'\n'HTTP_STATUS:}"
  printf '%s' "${out%$'\n'HTTP_STATUS:*}"
}

# ---------------------------------------------------------------------------- #
# Binary detection
# ---------------------------------------------------------------------------- #
find_binary() {
  local name="$1"
  local env_var="$2"
  local candidates=(
    "${!env_var:-}"
    "${ROOT}/target/debug/${name}"
    "${ROOT}/target/release/${name}"
    "$(command -v "${name}" 2>/dev/null || true)"
  )
  for c in "${candidates[@]}"; do
    [[ -n "$c" && -x "$c" ]] && { printf '%s' "$c"; return 0; }
  done
  return 1
}

# ---------------------------------------------------------------------------- #
# Setup checks
# ---------------------------------------------------------------------------- #
sep "Setup"

AETHER_BIN="${AETHER_BIN:-}"
if ! AETHER_BIN="$(find_binary aether AETHER_BIN)"; then
  info "aether binary not found — trying cargo build..."
  if command -v cargo &>/dev/null; then
    cargo build --bin aether --manifest-path "${ROOT}/Cargo.toml" 2>&1 | tail -3
    AETHER_BIN="${ROOT}/target/debug/aether"
    [[ -x "$AETHER_BIN" ]] || die "cargo build failed — cannot find aether binary"
  else
    die "aether binary not found and cargo not available. Build the project first."
  fi
fi
info "aether binary: ${AETHER_BIN}"

# Derive BASE_URL from TEST_PORT if not set
BASE_URL="${BASE_URL:-http://localhost:${TEST_PORT}}"

# ---------------------------------------------------------------------------- #
# licensegen — build if needed
# ---------------------------------------------------------------------------- #
LICENSEGEN_BIN="${LICENSEGEN_BIN:-}"
HAS_LICENSEGEN=false
if LICENSEGEN_BIN="$(find_binary licensegen LICENSEGEN_BIN)"; then
  HAS_LICENSEGEN=true
  info "licensegen binary: ${LICENSEGEN_BIN}"
elif command -v cargo &>/dev/null; then
  info "Building licensegen..."
  if cargo build --bin licensegen --manifest-path "${ROOT}/Cargo.toml" 2>&1 | tail -3; then
    LICENSEGEN_BIN="${ROOT}/target/debug/licensegen"
    [[ -x "$LICENSEGEN_BIN" ]] && HAS_LICENSEGEN=true && info "licensegen binary: ${LICENSEGEN_BIN}"
  fi
fi

PRIVATE_KEY="${ROOT}/keys/zyvor_private.pem"

# ---------------------------------------------------------------------------- #
# License generation
# ---------------------------------------------------------------------------- #
generate_license() {
  # Args: output_path valid_from valid_until
  local out="$1" from="$2" until="$3"
  if [[ "$HAS_LICENSEGEN" == true && -f "$PRIVATE_KEY" ]]; then
    "$LICENSEGEN_BIN" \
      --customer "Test Corp" \
      --customer-id test-corp \
      --allowed-nodes 100 \
      --allowed-clusters 5 \
      --valid-from "$from" \
      --valid-until "$until" \
      --private-key "$PRIVATE_KEY" \
      --out "$out" &>/dev/null
    return $?
  else
    # Static fallback: write a structurally valid but unsigned .zyvor file.
    # The server will accept it only if the embedded public key matches — if not,
    # the state will be INVALID_SIGNATURE rather than VALID/EXPIRED_GRACE.
    # This path is used only when neither licensegen nor the private key is available.
    warn "licensegen or private key unavailable — writing static (unsigned) .zyvor placeholder"
    warn "Valid-license tests may report INVALID_SIGNATURE instead of VALID"
    local claims
    claims="$(printf '%s' "{\"license_id\":\"TEST-001\",\"product\":\"zeus-os\",\"customer\":\"Test Corp\",\"customer_id\":\"test-corp\",\"allowed_nodes\":100,\"allowed_clusters\":5,\"valid_from\":\"${from}\",\"valid_until\":\"${until}\",\"issued_at\":\"2026-01-01\",\"license_version\":1}")"
    local b64_payload
    if command -v python3 &>/dev/null; then
      b64_payload="$(printf '%s' "$claims" | python3 -c 'import sys,base64; print(base64.urlsafe_b64encode(sys.stdin.buffer.read()).rstrip(b"=").decode())')"
    elif command -v openssl &>/dev/null; then
      b64_payload="$(printf '%s' "$claims" | openssl base64 -A | tr '+/' '-_' | tr -d '=')"
    else
      warn "Cannot base64-encode claims — writing empty placeholder"
      printf '{"format":"zyvor-v1","payload":"","signature":""}' > "$out"
      return 0
    fi
    printf '{"format":"zyvor-v1","payload":"%s","signature":"INVALID"}' "$b64_payload" > "$out"
    return 0
  fi
}

sep "Generating test licenses"

VALID_LICENSE_OK=false
EXPIRED_LICENSE_OK=false

if generate_license "$TEST_LICENSE" "2026-01-01" "2099-12-31"; then
  [[ -f "$TEST_LICENSE" ]] && VALID_LICENSE_OK=true && info "Valid license written: ${TEST_LICENSE}"
else
  warn "Failed to generate valid test license"
fi

if generate_license "$EXPIRED_LICENSE" "2020-01-01" "2021-01-01"; then
  [[ -f "$EXPIRED_LICENSE" ]] && EXPIRED_LICENSE_OK=true && info "Expired license written: ${EXPIRED_LICENSE}"
else
  warn "Failed to generate expired test license"
fi

# ---------------------------------------------------------------------------- #
# Server lifecycle
# ---------------------------------------------------------------------------- #
start_server() {
  local license_path="${1:-}"
  stop_server 2>/dev/null || true

  local env_prefix=()
  if [[ -n "$license_path" ]]; then
    env_prefix=(env "ZEUS_LICENSE_PATH=${license_path}")
  else
    # Unset ZEUS_LICENSE_PATH so the server uses the default (non-existent) path
    env_prefix=(env -u ZEUS_LICENSE_PATH)
  fi

  local api_key_env=()
  [[ -n "${AETHER_API_KEY:-}" ]] && api_key_env=("AETHER_API_KEY=${AETHER_API_KEY}")

  "${env_prefix[@]}" "${api_key_env[@]+"${api_key_env[@]}"}" \
    "$AETHER_BIN" serve --port "$TEST_PORT" \
    >"$SERVER_LOG" 2>&1 &
  SERVER_PID=$!
}

stop_server() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    SERVER_PID=""
  fi
}

wait_for_health() {
  local deadline=$(( $(date +%s) + 20 ))
  local url="${BASE_URL}/health"
  while [[ $(date +%s) -lt $deadline ]]; do
    if curl -sf "$url" &>/dev/null; then
      return 0
    fi
    sleep 0.5
  done
  warn "Server did not become healthy within 20s. Last 20 lines of log:"
  tail -20 "$SERVER_LOG" >&2
  return 1
}

# ---------------------------------------------------------------------------- #
# Cleanup trap
# ---------------------------------------------------------------------------- #
cleanup() {
  stop_server
  rm -f "$SERVER_LOG" "$TEST_LICENSE" "$EXPIRED_LICENSE"
}
trap cleanup EXIT INT TERM

# ---------------------------------------------------------------------------- #
# ===  TEST SUITE 1: Valid license  ==========================================
# ---------------------------------------------------------------------------- #
sep "Suite 1 — Valid license (server with ZEUS_LICENSE_PATH=${TEST_LICENSE})"

if [[ "$VALID_LICENSE_OK" != true ]]; then
  warn "Skipping Suite 1: valid license could not be generated"
else
  start_server "$TEST_LICENSE"
  if ! wait_for_health; then
    fail "Server health check timed out"
  else
    info "Server is up (PID ${SERVER_PID})"

    # ---- Test 1.1: GET /api/license/status --------------------------------- #
    body="$(do_curl GET "${BASE_URL}/api/license/status")"
    state="$(jq_get "$body" '.data.state')"
    customer="$(jq_get "$body" '.data.customer')"

    if [[ "$state" == "VALID" ]]; then
      pass "GET /api/license/status — state is VALID"
    else
      fail "GET /api/license/status — expected VALID, got '${state}' (body: ${body})"
    fi

    if [[ -n "$customer" && "$customer" != "null" ]]; then
      pass "GET /api/license/status — customer field present ('${customer}')"
    else
      fail "GET /api/license/status — customer field missing or null (body: ${body})"
    fi

    # ---- Test 1.2: GET /api/license/usage ---------------------------------- #
    body="$(do_curl GET "${BASE_URL}/api/license/usage")"
    nodes_check="$(jq_get "$body" '.data.nodes' 2>/dev/null || true)"
    util_pct="$(jq_get "$body" '.data.utilization_pct' 2>/dev/null || true)"

    # nodes must be a JSON array (check via jq or type heuristic)
    if command -v jq &>/dev/null; then
      nodes_type="$(printf '%s' "$body" | jq -r '.data.nodes | type' 2>/dev/null || true)"
      if [[ "$nodes_type" == "array" ]]; then
        pass "GET /api/license/usage — 'nodes' is an array"
      else
        fail "GET /api/license/usage — 'nodes' is not an array (type=${nodes_type})"
      fi
    else
      if printf '%s' "$body" | grep -q '"nodes":\s*\['; then
        pass "GET /api/license/usage — 'nodes' array present"
      else
        fail "GET /api/license/usage — 'nodes' array not found in response"
      fi
    fi

    if [[ -n "$util_pct" && "$util_pct" != "null" ]]; then
      pass "GET /api/license/usage — utilization_pct present (${util_pct})"
    else
      fail "GET /api/license/usage — utilization_pct missing or null (body: ${body})"
    fi

    # ---- Test 1.3: POST /api/license/reload -------------------------------- #
    # This endpoint requires Admin RBAC; skip gracefully if auth is rejected.
    body="$(do_curl POST "${BASE_URL}/api/license/reload")"
    reloaded="$(jq_get "$body" '.data.reloaded' 2>/dev/null || true)"

    if [[ "$CURL_STATUS" == "403" || "$CURL_STATUS" == "401" ]]; then
      if [[ -n "${AETHER_API_KEY:-}" ]]; then
        fail "POST /api/license/reload — auth rejected (HTTP ${CURL_STATUS}) despite AETHER_API_KEY being set"
      else
        warn "POST /api/license/reload — skipped (HTTP ${CURL_STATUS}: auth required; set AETHER_API_KEY to run this test)"
      fi
    elif [[ "$reloaded" == "true" ]]; then
      pass "POST /api/license/reload — reloaded:true"
    else
      fail "POST /api/license/reload — expected reloaded:true, got '${reloaded}' (HTTP ${CURL_STATUS}, body: ${body})"
    fi
  fi

  stop_server
fi

# ---------------------------------------------------------------------------- #
# ===  TEST SUITE 2: Missing license  ========================================
# ---------------------------------------------------------------------------- #
sep "Suite 2 — Missing license (server without ZEUS_LICENSE_PATH)"

start_server ""  # empty string => unset ZEUS_LICENSE_PATH in start_server
if ! wait_for_health; then
  fail "Server health check timed out (missing-license scenario)"
else
  info "Server is up (PID ${SERVER_PID})"

  body="$(do_curl GET "${BASE_URL}/api/license/status")"
  state="$(jq_get "$body" '.data.state')"

  if [[ "$state" == "MISSING" ]]; then
    pass "GET /api/license/status — state is MISSING (no license path set)"
  else
    fail "GET /api/license/status — expected MISSING, got '${state}' (body: ${body})"
  fi

  stop_server
fi

# ---------------------------------------------------------------------------- #
# ===  TEST SUITE 3: Expired license  ========================================
# ---------------------------------------------------------------------------- #
sep "Suite 3 — Expired license (valid_until 2021-01-01)"

if [[ "$EXPIRED_LICENSE_OK" != true ]]; then
  warn "Skipping Suite 3: expired license could not be generated"
else
  start_server "$EXPIRED_LICENSE"
  if ! wait_for_health; then
    fail "Server health check timed out (expired-license scenario)"
  else
    info "Server is up (PID ${SERVER_PID})"

    body="$(do_curl GET "${BASE_URL}/api/license/status")"
    state="$(jq_get "$body" '.data.state')"

    if [[ "$state" == "EXPIRED_GRACE" ]]; then
      pass "GET /api/license/status — state is EXPIRED_GRACE"
    else
      fail "GET /api/license/status — expected EXPIRED_GRACE, got '${state}' (body: ${body})"
    fi

    stop_server
  fi
fi

# ---------------------------------------------------------------------------- #
# Summary
# ---------------------------------------------------------------------------- #
sep "Results"
TOTAL=$(( PASS + FAIL ))
printf "\n  Passed: ${GREEN}%d${RESET} / %d\n" "$PASS" "$TOTAL"
[[ $FAIL -gt 0 ]] && printf "  Failed: ${RED}%d${RESET} / %d\n" "$FAIL" "$TOTAL"
printf "\n"

if [[ $FAIL -gt 0 ]]; then
  printf "${RED}FAIL${RESET} — %d test(s) failed\n" "$FAIL"
  exit 1
else
  printf "${GREEN}PASS${RESET} — all %d test(s) passed\n" "$PASS"
  exit 0
fi
