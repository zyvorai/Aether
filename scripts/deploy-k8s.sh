#!/usr/bin/env bash
# ============================================================================
# deploy-k8s.sh — Build and deploy Aether to the current Kubernetes cluster
# ============================================================================
# Usage:
#   ./scripts/deploy-k8s.sh [command]
# Commands:
#   detect   Show detected environment
#   build    Build container image locally
#   load     Load image directly into the current cluster runtime
#   deploy   Apply Kubernetes manifests
#   all      Build + load + deploy (default)
#   status   Show deployment status
#   logs     Tail deployment logs
#   delete   Remove Aether from the cluster
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

REPO="${AETHER_REGISTRY:-localhost/aether}"
VERSION="${VERSION:-$(grep '^version' "${REPO_ROOT}/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')}"
IMAGE="${REPO}:${VERSION}"
IMAGE_LATEST="${REPO}:latest"
NAMESPACE="${AETHER_NAMESPACE:-aether-system}"
NODE_PORT="${AETHER_NODE_PORT:-30090}"
AETHER_API_KEY="${AETHER_API_KEY:-}"

RUNTIME=""
KUBECTL="${KUBECTL:-kubectl}"
DISTRO="unknown"

detect_container_runtime() {
  if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    RUNTIME="docker"
  elif command -v podman >/dev/null 2>&1; then
    RUNTIME="podman"
  elif command -v nerdctl >/dev/null 2>&1; then
    RUNTIME="nerdctl"
  else
    echo "No supported container runtime found (docker, podman, nerdctl)" >&2
    exit 1
  fi
}

detect_k8s_distro() {
  local labels provider kubelet
  labels="$(${KUBECTL} get nodes -o jsonpath='{.items[0].metadata.labels}' 2>/dev/null || echo '')"
  provider="$(${KUBECTL} get nodes -o jsonpath='{.items[0].spec.providerID}' 2>/dev/null || echo '')"
  kubelet="$(${KUBECTL} get nodes -o jsonpath='{.items[0].status.nodeInfo.kubeletVersion}' 2>/dev/null || echo '')"
  if echo "${kubelet}" | grep -qi 'k3s'; then
    DISTRO="k3s"
  elif echo "${labels}" | grep -q 'minikube'; then
    DISTRO="minikube"
  elif echo "${provider}" | grep -q 'kind://'; then
    DISTRO="kind"
  elif echo "${labels}" | grep -q 'microk8s'; then
    DISTRO="microk8s"
  else
    DISTRO="k8s"
  fi
}

apply_manifests() {
  local deploy_stamp
  deploy_stamp="$(date +%s)-${RANDOM}"
  local api_key_secret=""
  local api_key_env=""
  if [ -n "${AETHER_API_KEY}" ]; then
    api_key_secret=$(cat <<EOF
---
apiVersion: v1
kind: Secret
metadata:
  name: aether-api-key
  namespace: ${NAMESPACE}
type: Opaque
stringData:
  api-key: ${AETHER_API_KEY}
EOF
)
    api_key_env=$(cat <<'EOF'
        - name: AETHER_API_KEY
          valueFrom:
            secretKeyRef:
              name: aether-api-key
              key: api-key
EOF
)
  fi

  ${KUBECTL} create namespace "${NAMESPACE}" --dry-run=client -o yaml | ${KUBECTL} apply -f -
  cat <<EOF | ${KUBECTL} apply -f -
apiVersion: v1
kind: ServiceAccount
metadata:
  name: aether
  namespace: ${NAMESPACE}
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: aether-discovery
rules:
- apiGroups: ['apps']
  resources: ['deployments', 'statefulsets', 'daemonsets']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['batch']
  resources: ['jobs', 'cronjobs']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['autoscaling']
  resources: ['horizontalpodautoscalers']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['networking.k8s.io']
  resources: ['ingresses', 'networkpolicies']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['discovery.k8s.io']
  resources: ['endpointslices']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['storage.k8s.io']
  resources: ['storageclasses']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['kubevirt.io']
  resources: ['virtualmachines', 'virtualmachineinstances']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['cdi.kubevirt.io']
  resources: ['datavolumes']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['']
  resources: ['pods', 'pods/log', 'services', 'configmaps', 'secrets', 'nodes', 'namespaces', 'persistentvolumes', 'persistentvolumeclaims', 'resourcequotas', 'limitranges', 'serviceaccounts', 'events', 'endpoints']
  verbs: ['get', 'list', 'watch', 'create', 'update', 'patch', 'delete']
- apiGroups: ['apiextensions.k8s.io']
  resources: ['customresourcedefinitions']
  verbs: ['get', 'list', 'watch']
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: aether-discovery
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: aether-discovery
subjects:
- kind: ServiceAccount
  name: aether
  namespace: ${NAMESPACE}
${api_key_secret}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aether
  namespace: ${NAMESPACE}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: aether
  template:
    metadata:
      labels:
        app: aether
      annotations:
        aether.dev/deploy-stamp: '${deploy_stamp}'
    spec:
      serviceAccountName: aether
      containers:
      - name: aether
        image: ${IMAGE_LATEST}
        imagePullPolicy: Never
        args: ['serve', '--host=0.0.0.0', '--port=5090']
        ports:
        - containerPort: 5090
          name: http
        env:
        - name: AETHER_LOG_FORMAT
          value: json
        - name: RUST_LOG
          value: info
${api_key_env}
        readinessProbe:
          httpGet:
            path: /health
            port: 5090
          initialDelaySeconds: 5
          periodSeconds: 10
        livenessProbe:
          httpGet:
            path: /health
            port: 5090
          initialDelaySeconds: 10
          periodSeconds: 30
---
apiVersion: v1
kind: Service
metadata:
  name: aether
  namespace: ${NAMESPACE}
spec:
  type: NodePort
  selector:
    app: aether
  ports:
  - port: 5090
    targetPort: 5090
    nodePort: ${NODE_PORT}
    protocol: TCP
    name: http
EOF
}

cmd_detect() {
  detect_container_runtime
  detect_k8s_distro
  echo "Container runtime: ${RUNTIME}"
  echo "Kubectl:           ${KUBECTL}"
  echo "K8s distro:        ${DISTRO}"
  echo "Namespace:         ${NAMESPACE}"
  echo "Image:             ${IMAGE_LATEST}"
}

cmd_build() {
  detect_container_runtime
  local extra=()
  [ "${RUNTIME}" = "podman" ] && extra+=(--format docker)
  ${RUNTIME} build "${extra[@]}" -t "${IMAGE}" -t "${IMAGE_LATEST}" "${REPO_ROOT}"
}

cmd_load() {
  detect_container_runtime
  detect_k8s_distro
  case "${DISTRO}" in
    k3s)
      local tarball
      tarball="$(mktemp /tmp/aether-image-XXXXXX.tar)"
      trap 'rm -f "${tarball}"' EXIT
      ${RUNTIME} save "${IMAGE_LATEST}" -o "${tarball}"
      sudo k3s ctr images import "${tarball}"
      ;;
    kind)
      if [ "${RUNTIME}" = "docker" ]; then
        kind load docker-image "${IMAGE_LATEST}"
      else
        local tarball
        tarball="$(mktemp /tmp/aether-image-XXXXXX.tar)"
        trap 'rm -f "${tarball}"' EXIT
        ${RUNTIME} save "${IMAGE_LATEST}" -o "${tarball}"
        kind load image-archive "${tarball}"
      fi
      ;;
    minikube)
      minikube image load "${IMAGE_LATEST}"
      ;;
    microk8s)
      local tarball
      tarball="$(mktemp /tmp/aether-image-XXXXXX.tar)"
      trap 'rm -f "${tarball}"' EXIT
      ${RUNTIME} save "${IMAGE_LATEST}" -o "${tarball}"
      microk8s ctr image import "${tarball}"
      ;;
    *)
      echo "Direct image loading is not supported for ${DISTRO}; use a registry-backed image flow." >&2
      exit 1
      ;;
  esac
}

cmd_deploy() {
  apply_manifests
  ${KUBECTL} -n "${NAMESPACE}" rollout restart deployment/aether >/dev/null 2>&1 || true
  ${KUBECTL} -n "${NAMESPACE}" rollout status deployment/aether --timeout=180s
}

cmd_status() {
  ${KUBECTL} -n "${NAMESPACE}" get pods,svc
}

cmd_logs() {
  ${KUBECTL} -n "${NAMESPACE}" logs deployment/aether -f
}

cmd_delete() {
  ${KUBECTL} delete namespace "${NAMESPACE}" --ignore-not-found
  ${KUBECTL} delete clusterrole aether-discovery --ignore-not-found
  ${KUBECTL} delete clusterrolebinding aether-discovery --ignore-not-found
}

COMMAND="${1:-all}"
case "${COMMAND}" in
  detect) cmd_detect ;;
  build) cmd_build ;;
  load) cmd_load ;;
  deploy) cmd_deploy ;;
  status) cmd_status ;;
  logs) cmd_logs ;;
  delete) cmd_delete ;;
  all)
    cmd_build
    cmd_load
    cmd_deploy
    cmd_status
    ;;
  *)
    echo "Unknown command: ${COMMAND}" >&2
    exit 1
    ;;
esac
