#!/usr/bin/env bash
# Reference-cluster E2E — Metal3, KubeVirt, and confidential offline/API checks.
#
# Usage:
#   ./scripts/reference-cluster-e2e.sh
#   AETHER_LABS_LIVE=1 KUBECONFIG=~/.kube/config ./scripts/reference-cluster-e2e.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

AETHER="${AETHER_BIN:-./target/release/aether}"
if [ ! -x "${AETHER}" ] && [ -x "./target/debug/aether" ]; then
  AETHER="./target/debug/aether"
fi
[ -x "${AETHER}" ] || { echo "Build aether first: make release" >&2; exit 1; }

echo "==> Validate confidential kata workload example"
"${AETHER}" --spec examples/confidential-kata-workload.yaml validate

echo "==> Labs E2E (Metal3 / KubeVirt dry-run or live when AETHER_LABS_LIVE=1)"
chmod +x scripts/labs-e2e.sh
AETHER_BIN="${AETHER}" scripts/labs-e2e.sh

if [ -f scripts/confidential-cluster-e2e.sh ]; then
  echo "==> Confidential cluster E2E (API offline checks)"
  chmod +x scripts/confidential-cluster-e2e.sh scripts/lib/aether-confidential-smoke.sh
  AETHER_TEE_SNP=1 AETHER_BIN="${AETHER}" SPEC=examples/confidential-snp.yaml scripts/confidential-cluster-e2e.sh
fi

echo "Reference cluster E2E complete."
