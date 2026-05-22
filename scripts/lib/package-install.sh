#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
# shellcheck source=/dev/null
[[ -f "${ROOT}/.package-lib/package-ui.sh" ]] && source "${ROOT}/.package-lib/package-ui.sh"

pkg_parse_install_args "$@"

_PKG_SESSION_START=${SECONDS}
pkg_install_welcome "Aether"
pkg_banner "Aether" "Workload platform · client bundle"
pkg_step_init 4

pkg_step "System dependencies"
[[ -x ./install-client-deps.sh ]] && { ./install-client-deps.sh || pkg_warn "deps issues"; pkg_step_done; } || { pkg_skip "install-client-deps.sh"; pkg_step_done; }

pkg_step "Configuration & Kubernetes access"
pkg_k8s_env_configure aether.env.example aether.env "Aether"
pkg_step_done

pkg_step "Verify binary"
[[ -x ./aether ]] && pkg_ok "aether (UI embedded)" || { pkg_fail "aether missing"; exit 1; }
pkg_step_done

pkg_step "Smoke test"
[[ -x ./test-package.sh ]] && ./test-package.sh || pkg_warn "test-package.sh"
pkg_step_done

pkg_install_finish "Aether" https 5090 "/web/dashboard/" \
  "Start: ./aether serve --host 0.0.0.0 --port 5090" \
  "Kubeconfig: ./install.sh --kubeconfig /path/to/config" \
  "Remove: ./uninstall.sh --yes [--remove-dir]"
