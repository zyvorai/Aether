#!/usr/bin/env bash
# Live KubeVirt reference-cluster smoke (requires kubeconfig + AETHER_LABS_LIVE=1).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

export AETHER_LABS_LIVE="${AETHER_LABS_LIVE:-1}"
AETHER="${AETHER_BIN:-./target/release/aether}"

echo "==> Labs live smoke (AETHER_LABS_LIVE=${AETHER_LABS_LIVE})"

if [[ -z "${KUBECONFIG:-}" ]] && [[ ! -f "${HOME}/.kube/config" ]]; then
  echo "ERROR: kubeconfig required for live labs smoke" >&2
  echo "  export KUBECONFIG=~/.kube/config" >&2
  exit 1
fi

if [[ ! -x "${AETHER}" ]]; then
  cargo build --release
fi

chmod +x scripts/reference-cluster-e2e.sh scripts/labs-e2e.sh
AETHER_BIN="${AETHER}" scripts/reference-cluster-e2e.sh

echo "==> Labs live smoke passed"
