#!/usr/bin/env bash
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
export PKG_INSTALL_ROOT="${ROOT}"
# shellcheck source=/dev/null
[[ -f "${ROOT}/.package-lib/package-ui.sh" ]] && source "${ROOT}/.package-lib/package-ui.sh"

[[ "${1:-}" == "-h" || "${1:-}" == "--help" ]] && {
  pkg_script_help "test-package.sh"
  exit 0
}

pkg_counters_reset
pkg_banner "Aether package test" "CLI · optional API"

[[ -x ./aether ]] && pkg_ok "aether" || pkg_fail "aether"
./aether --help >/dev/null 2>&1 && pkg_ok "aether --help" || pkg_fail "aether --help"

if curl -skf https://127.0.0.1:5090/health >/dev/null 2>&1; then
  pkg_ok "health :5090"
else
  pkg_skip "API not listening — ./aether serve --host 0.0.0.0 --port 5090"
fi

pkg_summary "Package test"
[[ "${_PKG_COUNTERS_FAIL}" -eq 0 ]]
