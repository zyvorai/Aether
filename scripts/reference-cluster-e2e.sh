#!/usr/bin/env bash
# Reference-cluster E2E — KubeVirt / labs offline checks.
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

echo "==> Labs E2E (KubeVirt dry-run or live when AETHER_LABS_LIVE=1)"
chmod +x scripts/labs-e2e.sh
AETHER_BIN="${AETHER}" scripts/labs-e2e.sh

echo "Reference cluster E2E complete."
