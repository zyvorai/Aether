#!/usr/bin/env bash
# Bootstrap curl auth for post-deploy verification scripts.
#
# Usage:
#   source scripts/lib/post-deploy-auth.sh
#   post_deploy_auth_prepare "${AETHER_API:-http://127.0.0.1:5090}"
#   curl "${AUTH[@]}" "${COOKIE_ARGS[@]}" "${AETHER_API}/api/server"
#
# Supports:
#   - AETHER_API_KEY bearer token
#   - Open API (no auth configured)
#   - Mock IdP SAML session cookie (AETHER_MOCK_IDP=1 on server)

post_deploy_auth_prepare() {
  local api="${1:?API base URL required}"
  local created_jar=0
  if [ -z "${AETHER_POST_DEPLOY_COOKIE_JAR:-}" ]; then
    AETHER_POST_DEPLOY_COOKIE_JAR="$(mktemp)"
    created_jar=1
  fi
  export AETHER_POST_DEPLOY_COOKIE_JAR
  AUTH=()
  COOKIE_ARGS=()
  if [ "${created_jar}" = "1" ]; then
    trap 'rm -f "${AETHER_POST_DEPLOY_COOKIE_JAR:-}"' EXIT
  fi

  if [ -n "${AETHER_API_KEY:-}" ]; then
    AUTH=(-H "Authorization: Bearer ${AETHER_API_KEY}")
    return 0
  fi

  if [ -s "${AETHER_POST_DEPLOY_COOKIE_JAR}" ]; then
    COOKIE_ARGS=(-b "${AETHER_POST_DEPLOY_COOKIE_JAR}" -c "${AETHER_POST_DEPLOY_COOKIE_JAR}")
    local cached_code
    cached_code="$(curl -sS -m 10 "${COOKIE_ARGS[@]}" -o /dev/null -w '%{http_code}' "${api}/api/server")"
    if [ "${cached_code}" = "200" ]; then
      return 0
    fi
    COOKIE_ARGS=()
  fi

  local code
  code="$(curl -sS -m 10 -o /dev/null -w '%{http_code}' "${api}/api/server")"
  if [ "${code}" = "200" ]; then
    return 0
  fi
  if [ "${code}" != "401" ]; then
    echo "WARN: /api/server returned ${code}; proceeding without auth bootstrap" >&2
    return 0
  fi

  local html_file
  html_file="$(mktemp)"
  if ! curl -sS -m 20 -L -o "${html_file}" "${api}/api/auth/saml/login"; then
    echo "ERROR: SAML login bootstrap failed" >&2
    rm -f "${html_file}"
    return 1
  fi

  if ! python3 - "${api}" "${AETHER_POST_DEPLOY_COOKIE_JAR}" "${html_file}" <<'PY'
import re
import sys
import urllib.parse
import urllib.request
import http.cookiejar
from pathlib import Path

api = sys.argv[1]
jar_path = sys.argv[2]
html = Path(sys.argv[3]).read_text()

match = re.search(r'name="SAMLResponse" value="([^"]+)"', html)
if not match:
    sys.stderr.write(
        "post-deploy-auth: SAMLResponse not found "
        "(set AETHER_API_KEY or run server with AETHER_MOCK_IDP=1)\n"
    )
    sys.exit(1)

relay = "/"
relay_match = re.search(r'name="RelayState" value="([^"]*)"', html)
if relay_match:
    relay = relay_match.group(1)

payload = urllib.parse.urlencode(
    {"SAMLResponse": match.group(1), "RelayState": relay}
).encode()
cookie_jar = http.cookiejar.MozillaCookieJar(jar_path)
opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(cookie_jar))
request = urllib.request.Request(f"{api}/api/auth/saml/acs", data=payload, method="POST")
request.add_header("Content-Type", "application/x-www-form-urlencoded")
try:
    with opener.open(request, timeout=20) as response:
        response.read()
except urllib.error.HTTPError as error:
    if error.code not in (302, 303, 307):
        raise
cookie_jar.save(ignore_discard=True, ignore_expires=True)
PY
  then
    rm -f "${html_file}"
    return 1
  fi
  rm -f "${html_file}"

  COOKIE_ARGS=(-b "${AETHER_POST_DEPLOY_COOKIE_JAR}" -c "${AETHER_POST_DEPLOY_COOKIE_JAR}")
  code="$(curl -sS -m 10 "${COOKIE_ARGS[@]}" -o /dev/null -w '%{http_code}' "${api}/api/server")"
  if [ "${code}" != "200" ]; then
    echo "ERROR: auth bootstrap failed (/api/server → ${code})" >&2
    return 1
  fi
  echo "  ok: session auth bootstrapped (mock IdP SAML)" >&2
}

post_deploy_auth_cookie_header() {
  python3 - "${AETHER_POST_DEPLOY_COOKIE_JAR:-}" <<'PY'
import http.cookiejar
import sys

jar_path = sys.argv[1]
if not jar_path:
    sys.exit(0)
jar = http.cookiejar.MozillaCookieJar(jar_path)
try:
    jar.load(ignore_discard=True, ignore_expires=True)
except FileNotFoundError:
    sys.exit(0)
header = "; ".join(f"{cookie.name}={cookie.value}" for cookie in jar)
if header:
    print(header)
PY
}
