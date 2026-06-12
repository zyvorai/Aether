#!/usr/bin/env bash
# Create/reuse kind cluster for Playwright cluster exec + port-forward E2E.
set -euo pipefail

CLUSTER="${AETHER_KIND_CLUSTER:-kind-aether-playwright}"
export KUBECONFIG="${KUBECONFIG:-${HOME}/.kube/config}"

if ! command -v kind >/dev/null 2>&1; then
  echo "kind not installed — skip fixture (set AETHER_E2E_KIND=1 only when kind is available)" >&2
  exit 0
fi

if ! kind get clusters 2>/dev/null | grep -qx "${CLUSTER}"; then
  kind create cluster --name "${CLUSTER}"
fi

kubectl config use-context "kind-${CLUSTER}"
kubectl apply -f - <<'YAML'
apiVersion: v1
kind: Pod
metadata:
  name: nginx-playwright
  namespace: default
  labels:
    app: nginx-playwright
spec:
  containers:
  - name: nginx
    image: nginx:1.25-alpine
    ports:
    - containerPort: 80
    command: ["/bin/sh", "-c", "while true; do echo aether-exec-ok; sleep 3600; done"]
YAML

kubectl wait --for=condition=Ready pod/nginx-playwright -n default --timeout=120s
echo "kind-playwright-fixture ready: context=kind-${CLUSTER} pod=nginx-playwright"
