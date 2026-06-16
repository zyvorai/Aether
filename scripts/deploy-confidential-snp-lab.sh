#!/usr/bin/env bash
# Confidential SNP lab — validate specs and run offline/API smoke (no live SNP hardware required).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${AETHER_DEPLOY_ENV:-$ROOT/examples/deploy/confidential-snp.env.example}"
if [ -f "${ENV_FILE}" ]; then
  set -a
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  set +a
fi
export AETHER_TEE_SNP="${AETHER_TEE_SNP:-1}"
SPEC="${AETHER_CONFIDENTIAL_SPEC:-examples/confidential-snp.yaml}"
AETHER="${AETHER_BIN:-$ROOT/target/release/aether}"
if [ ! -x "${AETHER}" ]; then
  (cd "${ROOT}" && cargo build --release)
  AETHER="${ROOT}/target/release/aether"
fi
echo "==> Validate confidential SNP spec: ${SPEC}"
"${AETHER}" --spec "${SPEC}" validate
echo "==> Confidential cluster E2E (offline)"
chmod +x "${ROOT}/scripts/confidential-cluster-e2e.sh" "${ROOT}/scripts/lib/aether-confidential-smoke.sh" 2>/dev/null || true
AETHER_TEE_SNP=1 AETHER_BIN="${AETHER}" SPEC="${SPEC}" "${ROOT}/scripts/confidential-cluster-e2e.sh"
if [ "${AETHER_LABS_LIVE:-}" = "1" ]; then
  echo "==> Live confidential fabric smoke"
  AETHER_TEE_SNP=1 AETHER_BIN="${AETHER}" "${ROOT}/scripts/reference-cluster-e2e.sh"
fi
echo "Confidential SNP lab checks complete."
