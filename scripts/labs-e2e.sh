#!/usr/bin/env bash
# Live KubeVirt checks. Skips when no cluster credentials are present.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

AETHER="${AETHER_BIN:-./target/release/aether}"

run_lab() {
  local name="$1"
  local spec="$2"
  local runtime="$3"

  echo "== Lab: $name =="
  "$AETHER" --spec "$spec" validate
  "$AETHER" run --spec "$spec" --runtime "$runtime" --dry-run

  if [[ -z "${KUBECONFIG:-}" ]] && [[ ! -f "${HOME}/.kube/config" ]]; then
    echo "  (skip live deploy: no kubeconfig)"
    return 0
  fi

  if [[ "${AETHER_LABS_LIVE:-}" != "1" ]]; then
    echo "  (skip live deploy: set AETHER_LABS_LIVE=1 to run against cluster)"
    return 0
  fi

  "$AETHER" run --spec "$spec" --runtime "$runtime"
  echo "  live deploy OK"
  if [[ "${AETHER_LABS_CLEANUP:-1}" == "1" ]]; then
    local wl
    wl="$(grep -E '^  name:' "$spec" | head -1 | awk '{print $2}')"
    if [[ -n "$wl" ]]; then
      echo "  cleaning up workload $wl"
      "$AETHER" --yes stop --name "$wl" --runtime "$runtime" --cascade 2>/dev/null || true
    fi
  fi
}

if [[ ! -x "$AETHER" ]]; then
  cargo build --release
fi

run_lab "kubevirt" "examples/labs/kubevirt/workload.yaml" "kubevirt"

if [[ -x scripts/k8s-labs-e2e.sh ]] || [[ -f scripts/k8s-labs-e2e.sh ]]; then
  chmod +x scripts/k8s-labs-e2e.sh
  AETHER_BIN="$AETHER" scripts/k8s-labs-e2e.sh
fi

echo "Labs e2e finished."
