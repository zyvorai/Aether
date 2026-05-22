#!/usr/bin/env bash
# Aether — one-command client install (extracted tarball).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
# shellcheck source=/dev/null
[[ -f "${ROOT}/.package-lib/package-ui.sh" ]] && source "${ROOT}/.package-lib/package-ui.sh"

_PKG_SESSION_START=${SECONDS}
pkg_install_welcome "Aether"

pkg_banner "Aether client install" "Kubernetes dashboard · client bundle"
pkg_step_init 4

pkg_step "System dependencies"
if [[ -x ./install-client-deps.sh ]]; then
  ./install-client-deps.sh && pkg_step_done || { pkg_warn "deps had issues"; pkg_step_done; }
else
  pkg_skip "install-client-deps.sh not found"; pkg_step_done
fi

pkg_step "Configuration"
if [[ -f aether.env.example ]] && [[ ! -f aether.env ]]; then
  cp aether.env.example aether.env
  pkg_ok "Created aether.env — set KUBECONFIG"
elif [[ -f aether.env ]]; then
  pkg_ok "aether.env present"
else
  pkg_warn "aether.env.example missing"
fi
pkg_step_done

pkg_step "Verify binaries"
if [[ -x ./aether ]]; then
  ./aether --help >/dev/null 2>&1 && pkg_ok "aether --help" || pkg_warn "aether --help"
else
  pkg_fail "./aether missing"; exit 1
fi
pkg_step_done

pkg_step "Smoke test"
[[ -x ./test-package.sh ]] && ./test-package.sh || pkg_skip "test-package.sh"
pkg_step_done

pkg_summary "Install complete"
pkg_next_steps \
  "https://zyvor.dev · © @zyvor 2026" \
  "./aether serve --host 0.0.0.0 --port 5090" \
  "$(pkg_access_url https 5090)/web/dashboard/ ($(pkg_primary_host_label))" \
  "./test-package.sh" \
  "./uninstall.sh --yes [--remove-dir]"
