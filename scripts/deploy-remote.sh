#!/usr/bin/env bash
# ============================================================================
# deploy-remote.sh — Deploy Aether to a remote Kubernetes cluster
# ============================================================================
# Mirrors the VMRogue remote deploy shape:
#   1. Detect remote Kubernetes flavor + container/image runtime
#   2. Optionally rsync source to remote ~/.deployment/aether
#   3. Build release binary locally or remotely
#   4. Build + import localhost/aether:latest into the node runtime
#   5. Apply Kubernetes manifests and wait for rollout
#
# Usage:
#   ./scripts/deploy-remote.sh [flags] <host> [user]
#   ./scripts/deploy-remote.sh 185.165.240.5 sus
#
# Flags:
#   --skip-sync     Skip rsync (AETHER_SKIP_RSYNC=1)
#   --skip-cargo    Skip cargo build (AETHER_SKIP_CARGO=1)
#   --skip-image    Skip image build/import (AETHER_SKIP_IMAGE=1)
#   --quick         Skip sync + cargo + image build/import
#   --local-build   Build locally and upload only the release binary
#   --uninstall     Remove Aether from the remote cluster
#
# Environment:
#   AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP=1 — skip deploy/k8s/bootstrap/cilium-aether-egress.yaml
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"

SKIP_RSYNC="${AETHER_SKIP_RSYNC:-0}"
SKIP_CARGO="${AETHER_SKIP_CARGO:-0}"
SKIP_IMAGE="${AETHER_SKIP_IMAGE:-0}"
LOCAL_BUILD="${AETHER_LOCAL_BUILD:-0}"
UNINSTALL=0

POSITIONAL=()
for arg in "$@"; do
  case "$arg" in
    --skip-sync) SKIP_RSYNC=1 ;;
    --skip-cargo) SKIP_CARGO=1 ;;
    --skip-image) SKIP_IMAGE=1 ;;
    --quick)
      SKIP_RSYNC=1
      SKIP_CARGO=1
      SKIP_IMAGE=1
      ;;
    --local-build) LOCAL_BUILD=1 ;;
    --uninstall) UNINSTALL=1 ;;
    --help|-h)
      sed -n '2,28p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) POSITIONAL+=("$arg") ;;
  esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-root}}"
PASS="${DEPLOY_PASS:-}"

[ -n "${HOST}" ] || {
  echo "Usage: $0 [flags] <host> [user]"
  exit 1
}

AETHER_NS="${AETHER_NAMESPACE:-aether-system}"
AETHER_IMAGE="${AETHER_IMAGE:-localhost/aether:latest}"
NODE_PORT="${AETHER_NODE_PORT:-30090}"
AETHER_API_KEY="${AETHER_API_KEY:-}"
AETHER_LOG_FORMAT="${AETHER_LOG_FORMAT:-json}"

info() { aether_ok "$*"; }
warn() { aether_warn "$*"; }
step() { aether_step "$*"; }
error() { aether_die "$*"; }

if [ -n "${PASS}" ] && ! command -v sshpass >/dev/null 2>&1; then
  error "sshpass is required when DEPLOY_PASS is set"
fi

ssh_cmd() {
  if [ -n "${PASS}" ]; then
    SSHPASS="${PASS}" sshpass -e ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
  else
    ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
  fi
}

rsync_cmd() {
  local ssh_wrapper="ssh -o StrictHostKeyChecking=no"
  if [ -n "${PASS}" ]; then
    ssh_wrapper="sshpass -e ${ssh_wrapper}"
  fi
  SSHPASS="${PASS}" rsync -az --delete \
    --exclude '.git/' \
    --exclude 'target/' \
    --exclude 'node_modules/' \
    --exclude 'headlamp/' \
    -e "${ssh_wrapper}" \
    "$@"
}

REMOTE_HOME="$(ssh_cmd "echo \$HOME" | tr -d '\r')"
REMOTE_DIR="${REMOTE_HOME}/.deployment/aether"

detect_remote_env() {
  local result
  result="$(ssh_cmd '
    K8S_FLAVOR=none
    if [ -x /usr/local/bin/k3s ]; then
      K8S_FLAVOR=k3s_std
    elif command -v k3s >/dev/null 2>&1; then
      K8S_FLAVOR=k3s_path
    elif command -v kubectl >/dev/null 2>&1; then
      K8S_FLAVOR=kubectl
    fi

    BUILD_TOOL=none
    if command -v podman >/dev/null 2>&1; then
      BUILD_TOOL=podman
    elif command -v docker >/dev/null 2>&1; then
      BUILD_TOOL=docker
    elif command -v nerdctl >/dev/null 2>&1; then
      BUILD_TOOL=nerdctl
    fi

    echo "${K8S_FLAVOR}|${BUILD_TOOL}"
  ' | tr -d '\r')"

  REMOTE_K8S_FLAVOR="${result%%|*}"
  BUILD_TOOL="${result##*|}"

  case "${REMOTE_K8S_FLAVOR}" in
    k3s_std)
      K="sudo /usr/local/bin/k3s kubectl"
      IMPORT_CMD="${AETHER_CONTAINER_RUNTIME_IMPORT:-sudo /usr/local/bin/k3s ctr images import -}"
      ;;
    k3s_path)
      K="sudo k3s kubectl"
      IMPORT_CMD="${AETHER_CONTAINER_RUNTIME_IMPORT:-sudo k3s ctr images import -}"
      ;;
    kubectl)
      K="kubectl"
      IMPORT_CMD="${AETHER_CONTAINER_RUNTIME_IMPORT:-sudo ctr -n k8s.io images import -}"
      ;;
    *)
      error "No remote Kubernetes CLI found on ${USER}@${HOST}"
      ;;
  esac

  [ "${BUILD_TOOL}" != "none" ] || [ "${SKIP_IMAGE}" = "1" ] || error "No remote build tool found (need podman, docker, or nerdctl)"
}

detect_remote_env

aether_banner_orchestrator
aether_kv "Host" "${USER}@${HOST}"
aether_kv "Cluster" "${REMOTE_K8S_FLAVOR}"
aether_kv "Kubectl" "${K}"
aether_kv "Builder" "${BUILD_TOOL}"
aether_kv "Import" "${IMPORT_CMD}"
aether_kv "Namespace" "${AETHER_NS}"
aether_kv "Image" "${AETHER_IMAGE}"
aether_kv "NodePort" "${NODE_PORT}"
aether_kv "Fast path" "rsync=${SKIP_RSYNC} cargo=${SKIP_CARGO} image=${SKIP_IMAGE} local_build=${LOCAL_BUILD}"
echo ""

if [ "${UNINSTALL}" = "1" ]; then
  step "Removing Aether from remote cluster"
  ssh_cmd "
    ${K} delete namespace ${AETHER_NS} --ignore-not-found
    ${K} delete clusterrole aether-discovery --ignore-not-found
    ${K} delete clusterrolebinding aether-discovery --ignore-not-found
    rm -rf ${REMOTE_DIR}
  " >/dev/null 2>&1 || true
  info "Aether removed from ${HOST}"
  aether_finale_uninstall
  exit 0
fi

TOTAL_STEPS=5

step "Step 1/${TOTAL_STEPS}: Syncing source"
if [ "${SKIP_RSYNC}" = "1" ]; then
  info "Skipped rsync"
else
  ssh_cmd "mkdir -p ${REMOTE_DIR}"
  rsync_cmd "${REPO_ROOT}/" "${USER}@${HOST}:${REMOTE_DIR}/"
  info "Source synced to ${REMOTE_DIR}"
fi

step "Step 2/${TOTAL_STEPS}: Building release binary"
if [ "${LOCAL_BUILD}" = "1" ]; then
  (cd "${REPO_ROOT}" && cargo build --release)
  strip "${REPO_ROOT}/target/release/aether" 2>/dev/null || true
  ssh_cmd "mkdir -p ${REMOTE_DIR}/target/release"
  rsync_cmd "${REPO_ROOT}/target/release/aether" "${USER}@${HOST}:${REMOTE_DIR}/target/release/aether"
  info "Binary built locally and uploaded"
elif [ "${SKIP_CARGO}" = "1" ]; then
  ssh_cmd "test -x ${REMOTE_DIR}/target/release/aether" || error "No remote release binary at ${REMOTE_DIR}/target/release/aether"
  info "Skipped cargo build"
else
  ssh_cmd "
    source \$HOME/.cargo/env 2>/dev/null || true
    if ! command -v cargo >/dev/null 2>&1; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      source \$HOME/.cargo/env
    fi
    cd ${REMOTE_DIR}
    cargo build --release
    strip target/release/aether 2>/dev/null || true
  "
  info "Binary built on remote"
fi

step "Step 3/${TOTAL_STEPS}: Building and importing image"
if [ "${SKIP_IMAGE}" = "1" ]; then
  info "Skipped image build/import"
else
  ssh_cmd "
    cd ${REMOTE_DIR}
    cat > Dockerfile.deploy <<'EOF'
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl openssl && rm -rf /var/lib/apt/lists/*
COPY target/release/aether /usr/local/bin/aether
RUN mkdir -p /var/lib/aether /tls
EXPOSE 5090
ENTRYPOINT [\"/usr/local/bin/aether\"]
CMD [\"serve\", \"--host=0.0.0.0\", \"--port=5090\"]
EOF
    ${BUILD_TOOL} build --format docker -t ${AETHER_IMAGE} -f Dockerfile.deploy .
    ${BUILD_TOOL} save ${AETHER_IMAGE} | ${IMPORT_CMD}
  "
  info "Image built and imported"
fi

step "Step 4/${TOTAL_STEPS}: Applying Kubernetes manifests"
DEPLOY_STAMP="$(date +%s)-${RANDOM}"
API_KEY_SECRET_BLOCK=""
API_KEY_ENV_BLOCK=""
if [ -n "${AETHER_API_KEY}" ]; then
  API_KEY_SECRET_BLOCK=$(cat <<EOF
---
apiVersion: v1
kind: Secret
metadata:
  name: aether-api-key
  namespace: ${AETHER_NS}
type: Opaque
stringData:
  api-key: ${AETHER_API_KEY}
EOF
)
  API_KEY_ENV_BLOCK=$(cat <<'EOF'
        - name: AETHER_API_KEY
          valueFrom:
            secretKeyRef:
              name: aether-api-key
              key: api-key
EOF
)
fi

ssh_cmd "
  ${K} create namespace ${AETHER_NS} --dry-run=client -o yaml | ${K} apply -f -
  if [ \"${AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"1\" ] && [ \"${AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}\" != \"true\" ] && ${K} get crd ciliumnetworkpolicies.cilium.io &>/dev/null && [ -f \"${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-aether-egress.yaml\" ]; then
    sed \"s|__AETHER_NAMESPACE__|${AETHER_NS}|g\" \"${REMOTE_DIR}/deploy/k8s/bootstrap/cilium-aether-egress.yaml\" | ${K} apply -f -
    echo 'Applied CiliumNetworkPolicy allow-aether-egress'
  else
    echo 'Skipping Cilium aether egress bootstrap (AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP=1, no Cilium CRD, or yaml missing)'
  fi
  cat <<'EOF' | ${K} apply -f -
apiVersion: v1
kind: ServiceAccount
metadata:
  name: aether
  namespace: ${AETHER_NS}
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
  namespace: ${AETHER_NS}
${API_KEY_SECRET_BLOCK}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aether
  namespace: ${AETHER_NS}
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
        aether.dev/deploy-stamp: '${DEPLOY_STAMP}'
    spec:
      serviceAccountName: aether
      containers:
      - name: aether
        image: ${AETHER_IMAGE}
        imagePullPolicy: Never
        args: ['serve', '--host=0.0.0.0', '--port=5090']
        ports:
        - containerPort: 5090
          name: http
        env:
        - name: AETHER_LOG_FORMAT
          value: '${AETHER_LOG_FORMAT}'
        - name: RUST_LOG
          value: info
${API_KEY_ENV_BLOCK}
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
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: '1'
            memory: 512Mi
---
apiVersion: v1
kind: Service
metadata:
  name: aether
  namespace: ${AETHER_NS}
spec:
  type: NodePort
  selector:
    app: aether
  ports:
  - name: http
    port: 5090
    targetPort: 5090
    nodePort: ${NODE_PORT}
EOF
  ${K} -n ${AETHER_NS} rollout restart deployment/aether >/dev/null 2>&1 || true
  ${K} -n ${AETHER_NS} rollout status deployment/aether --timeout=180s
"
info "Kubernetes deployment applied"

step "Step 5/${TOTAL_STEPS}: Verifying"
ssh_cmd "
  ${K} -n ${AETHER_NS} get pods -o wide
  ${K} -n ${AETHER_NS} get svc aether
" || error "Verification failed"

echo ""
info "Aether is available at http://${HOST}:${NODE_PORT}"
if [ -n "${AETHER_API_KEY}" ]; then
  info "API authentication is enabled on the remote deployment"
fi
aether_subtle "  🚀 Remote path complete. Run ./scripts/health-check-all.sh from the bundle for a full seal."
