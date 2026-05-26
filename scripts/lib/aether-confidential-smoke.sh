#!/usr/bin/env bash
# Shared helpers for Aether confidential API smoke / cluster E2E scripts.
# Source from scripts/confidential-*.sh — do not execute directly.

: "${AETHER_API:=${AETHER_API_BASE:-http://127.0.0.1:5090}}"
: "${AETHER_API_KEY:=}"

_aether_auth_args() {
  if [ -n "${AETHER_API_KEY}" ]; then
    printf '%s' "-H" "Authorization: Bearer ${AETHER_API_KEY}"
  fi
}

_aether_curl() {
  # shellcheck disable=SC2046
  curl -sS -m "${AETHER_CURL_TIMEOUT:-30}" $(_aether_auth_args) "$@"
}

aether_http_code() {
  local path="$1"
  local method="${2:-GET}"
  # shellcheck disable=SC2046
  curl -sS -m "${AETHER_CURL_TIMEOUT:-30}" -o /dev/null -w '%{http_code}' \
    $(_aether_auth_args) -X "${method}" "${AETHER_API}${path}"
}

aether_fetch_json() {
  local path="$1"
  local out="$2"
  local code
  # shellcheck disable=SC2046
  code=$(curl -sS -m "${AETHER_CURL_TIMEOUT:-30}" $(_aether_auth_args) \
    -o "${out}" -w '%{http_code}' "${AETHER_API}${path}")
  printf '%s' "${code}"
}

aether_smoke_pass=0
aether_smoke_fail=0

aether_ok() {
  echo "  OK: $*"
  aether_smoke_pass=$((aether_smoke_pass + 1))
}

aether_fail() {
  echo "  FAIL: $*" >&2
  aether_smoke_fail=$((aether_smoke_fail + 1))
}

aether_warn() {
  echo "  WARN: $*"
}

aether_expect_http() {
  local label="$1"
  local path="$2"
  local want="${3:-200}"
  local method="${4:-GET}"
  local got
  got="$(aether_http_code "${path}" "${method}")"
  if [ "${got}" = "${want}" ]; then
    aether_ok "${label} (HTTP ${got})"
  else
    aether_fail "${label} (HTTP ${got}, expected ${want})"
  fi
}

aether_smoke_confidential_apis() {
  echo "── Aether confidential APIs (${AETHER_API}) ──"
  aether_expect_http "GET /health" "/health"
  aether_expect_http "GET /api/confidential/capabilities" "/api/confidential/capabilities"
  aether_expect_http "GET /api/confidential/fleet" "/api/confidential/fleet"
  aether_expect_http "GET /api/confidential/trust-score" "/api/confidential/trust-score"
  aether_expect_http "GET /api/confidential/intelligence" "/api/confidential/intelligence"
  aether_expect_http "GET /api/confidential/kata/status" "/api/confidential/kata/status"
  aether_expect_http "GET /api/confidential/sovereign/status" "/api/confidential/sovereign/status"
  aether_expect_http "GET /api/confidential/images" "/api/confidential/images"
  echo "  Smoke: ${aether_smoke_pass} passed, ${aether_smoke_fail} failed"
  [ "${aether_smoke_fail}" -eq 0 ]
}

aether_check_tee_capabilities() {
  local require_snp="${REQUIRE_SNP:-0}"
  local tmp
  tmp="$(mktemp)"
  local code
  code="$(aether_fetch_json "/api/confidential/capabilities" "${tmp}")"
  if [ "${code}" != "200" ]; then
    aether_fail "capabilities HTTP ${code}"
    rm -f "${tmp}"
    return 1
  fi
  python3 - "${tmp}" "${require_snp}" <<'PY'
import json, sys
path, require = sys.argv[1], sys.argv[2] == "1"
with open(path) as f:
    body = json.load(f)
data = body.get("data") or body
host = data.get("host") or {}
snp = bool(host.get("sev_snp"))
tdx = bool(host.get("tdx"))
print(f"  host sev_snp={snp} tdx={tdx}")
if require and not snp and not tdx:
    print("  FAIL: REQUIRE_SNP=1 but host reports no SNP/TDX", file=sys.stderr)
    sys.exit(1)
if snp:
    print("  OK: SEV-SNP advertised on control plane host")
elif require:
    print("  FAIL: SNP required", file=sys.stderr)
    sys.exit(1)
else:
    print("  WARN: no SNP on host (lab may set AETHER_TEE_SNP=1)")
PY
  local rc=$?
  rm -f "${tmp}"
  return "${rc}"
}

ragnarok_composite_check() {
  local base="${RAGNAROK_API:-}"
  [ -n "${base}" ] || return 0
  echo "── Ragnarok composite (${base}) ──"
  local tmp
  tmp="$(mktemp)"
  if ! curl -sS -m 15 -o "${tmp}" "${base%/}/api/v1/confidential/composite/status"; then
    aether_fail "Ragnarok composite/status unreachable"
    rm -f "${tmp}"
    return 1
  fi
  python3 - "${tmp}" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f).get("data", {})
if d.get("aether_url_configured"):
    print("  OK: Ragnarok aether_url_configured")
else:
    print("  WARN: Ragnarok composite without AETHER_URL")
if d.get("aether_reachable"):
    print("  OK: Ragnarok aether_reachable")
else:
    print("  WARN: Aether not reachable from Ragnarok")
PY
  rm -f "${tmp}"
}
