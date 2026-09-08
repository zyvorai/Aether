#!/usr/bin/env bash
# Live KubeVirt reference cluster smoke + optional post-deploy API verify.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

export AETHER_LABS_LIVE="${AETHER_LABS_LIVE:-1}"
AETHER="${AETHER_BIN:-./target/release/aether}"
API="${AETHER_API:-http://127.0.0.1:5090}"

echo "==> Reference cluster live verify (AETHER_LABS_LIVE=${AETHER_LABS_LIVE})"

if [[ -z "${KUBECONFIG:-}" ]] && [[ ! -f "${HOME}/.kube/config" ]]; then
  echo "ERROR: kubeconfig required — export KUBECONFIG=~/.kube/config" >&2
  exit 1
fi

chmod +x scripts/labs-live-smoke.sh scripts/reference-cluster-e2e.sh scripts/labs-e2e.sh
AETHER_BIN="${AETHER}" scripts/labs-live-smoke.sh

if curl -sf "${API}/health" >/dev/null 2>&1; then
  echo "==> Post-deploy API verify → ${API}"
  AETHER_API="${API}" scripts/post-deploy-verify.sh "${API}"
else
  echo "  (skip post-deploy verify: API not reachable at ${API})"
fi

echo "==> Reference cluster live verify passed"
