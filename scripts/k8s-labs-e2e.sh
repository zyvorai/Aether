#!/usr/bin/env bash
# Kubernetes lab: validate, dry-run, and optional live deploy (kind or any kubeconfig).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

AETHER="${AETHER_BIN:-./target/release/aether}"
SPEC="examples/labs/kubernetes/workload.yaml"
WORKLOAD="nginx"
NAMESPACE="${AETHER_NAMESPACE:-default}"

echo "== Lab: kubernetes =="
"$AETHER" --spec "$SPEC" validate
"$AETHER" run --spec "$SPEC" --runtime kube --dry-run

if [[ -z "${KUBECONFIG:-}" ]] && [[ ! -f "${HOME}/.kube/config" ]]; then
  echo "  (skip live deploy: no kubeconfig)"
  exit 0
fi

if [[ "${AETHER_LABS_LIVE:-}" != "1" ]]; then
  echo "  (skip live deploy: set AETHER_LABS_LIVE=1 to run against cluster)"
  exit 0
fi

if command -v kind >/dev/null 2>&1 && [[ "${AETHER_K8S_KIND_SETUP:-}" == "1" ]]; then
  if ! kind get clusters 2>/dev/null | grep -qx "aether-labs"; then
    echo "  creating kind cluster aether-labs..."
    kind create cluster --name aether-labs
  fi
  kubectl config use-context kind-aether-labs
  echo "  loading nginx:latest into kind..."
  docker pull nginx:alpine
  docker tag nginx:alpine nginx:latest
  kind load docker-image nginx:latest --name aether-labs
fi

export AETHER_NAMESPACE="$NAMESPACE"
"$AETHER" run --spec "$SPEC" --runtime kube

echo "  waiting for Deployment/nginx rollout..."
kubectl -n "$NAMESPACE" rollout status "deployment/$WORKLOAD" --timeout=120s

echo "  waiting for pod readiness..."
kubectl -n "$NAMESPACE" wait --for=condition=ready "pod" -l "app=$WORKLOAD" --timeout=120s

echo "  cleaning up workload..."
"$AETHER" delete "$WORKLOAD" || true

echo "  live kubernetes deploy OK"
