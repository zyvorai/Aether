#!/bin/bash
# ============================================================================
# deploy-remote.sh — Deploy Aether to a remote Kubernetes cluster
# ============================================================================
# Auto-detects k3s vs standard k8s, containerd vs docker, podman vs docker.
#
# Deploys Aether as a Kubernetes Deployment + Service + NodePort:
#   1. Detect cluster type and container runtime
#   2. Rsync repo to remote ~/.deployment/aether
#   3. Build container image (podman or docker)
#   4. Import image into cluster runtime
#   5. Apply K8s manifests (Namespace, Deployment, Service)
#   6. Clean up source code
#   7. Verify everything works
#
# Usage:
#   ./scripts/deploy-remote.sh <host> [user] [password]
#   ./scripts/deploy-remote.sh 185.165.240.5 sus Admin@321
#   ./scripts/deploy-remote.sh 185.165.240.5 sus             # uses default password
#   ./scripts/deploy-remote.sh 185.165.240.5 sus pass --quick # rebuild image only
#   ./scripts/deploy-remote.sh 185.165.240.5 sus pass --uninstall
# ============================================================================

set -euo pipefail

info()  { echo "  ✅ $*"; }
warn()  { echo "  ⚠️  $*"; }
error() { echo "  ❌ $*"; exit 1; }
step()  { echo ""; echo "  🔧 $*"; }

# ── Parse args ──
QUICK_MODE=false
UNINSTALL_MODE=false
POSITIONAL=()
for arg in "$@"; do
    case "$arg" in
        --quick)     QUICK_MODE=true ;;
        --uninstall) UNINSTALL_MODE=true ;;
        --help|-h)
            echo "Usage: $0 <host> [user] [password] [--quick|--uninstall]"
            echo ""
            echo "  Deploys Aether as a Kubernetes Deployment + NodePort Service."
            echo "  Auto-detects k3s vs standard Kubernetes and container runtime."
            echo ""
            echo "  --quick      Skip deps, only rebuild image + redeploy"
            echo "  --uninstall  Remove Aether from the cluster"
            echo ""
            exit 0
            ;;
        *)  POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-root}}"
PASS="${POSITIONAL[2]:-${DEPLOY_PASS:-Admin@321}}"

[ -z "$HOST" ] && error "Usage: $0 <host> [user] [password] [--quick]"

if [ "$USER" = "root" ]; then
    REMOTE_HOME="/root"
else
    REMOTE_HOME="/home/${USER}"
fi
REMOTE_DIR="${REMOTE_HOME}/.deployment/aether"

SUDO=""
[ "$USER" != "root" ] && SUDO="sudo"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

[ -f "$REPO_DIR/Cargo.toml" ] || error "Not in aether repo: $REPO_DIR"

AETHER_NS="aether-system"
AETHER_IMAGE="localhost/aether:latest"
NODE_PORT=30090

# ── SSH/rsync wrappers ──
_ssh() {
    if [ -n "$PASS" ]; then
        SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
    else
        ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
    fi
}

_rsync() {
    local ssh_cmd="ssh -o StrictHostKeyChecking=no"
    if [ -n "$PASS" ]; then
        ssh_cmd="sshpass -e $ssh_cmd"
    fi
    SSHPASS="$PASS" rsync -avz \
        --exclude='.git' --exclude='target/' \
        --exclude='*.o' --exclude='*.d' --exclude='*.rlib' \
        --exclude='*.rmeta' --exclude='*.fingerprint' \
        --exclude='node_modules/' \
        -e "$ssh_cmd" \
        "$@"
}

# ── Preflight ──
if [ -n "$PASS" ] && ! command -v sshpass &>/dev/null; then
    error "sshpass required for password auth. Install: dnf install sshpass"
fi

# ── Detect cluster type and container runtime ──
detect_cluster() {
    local result
    result=$(_ssh "
        K8S_TYPE='unknown'
        CRI='unknown'
        BUILD_TOOL='unknown'

        # Detect k3s vs standard k8s
        if [ -x /usr/local/bin/k3s ]; then
            K8S_TYPE='k3s'
        elif command -v kubeadm &>/dev/null; then
            K8S_TYPE='kubeadm'
        elif command -v kubectl &>/dev/null; then
            K8S_TYPE='k8s'
        fi

        # Detect container runtime from node info
        if command -v kubectl &>/dev/null; then
            CRI_RAW=\$(kubectl get nodes -o jsonpath='{.items[0].status.nodeInfo.containerRuntimeVersion}' 2>/dev/null || echo '')
            case \"\$CRI_RAW\" in
                containerd*)  CRI='containerd' ;;
                docker*)      CRI='docker' ;;
                cri-o*)       CRI='cri-o' ;;
                *)
                    # Fallback: probe directly
                    if [ \"\$K8S_TYPE\" = 'k3s' ]; then
                        CRI='containerd'
                    elif command -v docker &>/dev/null && docker info &>/dev/null 2>&1; then
                        CRI='docker'
                    elif command -v crictl &>/dev/null; then
                        CRI='containerd'
                    fi
                    ;;
            esac
        fi

        # Detect build tool
        if command -v podman &>/dev/null; then
            BUILD_TOOL='podman'
        elif command -v docker &>/dev/null; then
            BUILD_TOOL='docker'
        fi

        echo \"\${K8S_TYPE}|\${CRI}|\${BUILD_TOOL}\"
    " 2>&1 | tail -1)

    K8S_TYPE=$(echo "$result" | cut -d'|' -f1)
    CRI=$(echo "$result" | cut -d'|' -f2)
    BUILD_TOOL=$(echo "$result" | cut -d'|' -f3)

    if [ "$K8S_TYPE" = "unknown" ]; then error "No Kubernetes installation detected on ${HOST}"; fi
    if [ "$CRI" = "unknown" ]; then error "Could not detect container runtime on ${HOST}"; fi
    if [ "$BUILD_TOOL" = "unknown" ]; then error "No container build tool (podman/docker) found on ${HOST}"; fi
}

detect_cluster

# ── Uninstall mode ──
if $UNINSTALL_MODE; then
    echo ""
    echo "  ╔══════════════════════════════════════════════════════╗"
    echo "  ║     🗑️  Aether Kubernetes Uninstall               ║"
    echo "  ╚══════════════════════════════════════════════════════╝"
    echo ""
    echo "  Host:      ${USER}@${HOST}"
    echo "  Cluster:   ${K8S_TYPE} (${CRI})"
    echo "  Namespace: ${AETHER_NS}"
    echo ""

    step "Removing Aether from Kubernetes"
    if [ "$K8S_TYPE" = "k3s" ]; then
        _ssh "
            kubectl delete namespace ${AETHER_NS} --ignore-not-found 2>&1 || true
            kubectl delete clusterrole aether-discovery --ignore-not-found 2>&1 || true
            kubectl delete clusterrolebinding aether-discovery --ignore-not-found 2>&1 || true
            sudo /usr/local/bin/k3s ctr -n k8s.io images rm ${AETHER_IMAGE} 2>/dev/null || true
            rm -rf ${REMOTE_DIR}
            rm -rf ${REMOTE_HOME}/.deployment
            echo 'Done'
        " 2>&1
    else
        _ssh "
            kubectl delete namespace ${AETHER_NS} --ignore-not-found 2>&1 || true
            kubectl delete clusterrole aether-discovery --ignore-not-found 2>&1 || true
            kubectl delete clusterrolebinding aether-discovery --ignore-not-found 2>&1 || true
            if [ '${CRI}' = 'containerd' ]; then
                sudo ctr -n k8s.io images rm ${AETHER_IMAGE} 2>/dev/null || true
            elif [ '${CRI}' = 'docker' ]; then
                sudo docker rmi ${AETHER_IMAGE} 2>/dev/null || true
            fi
            rm -rf ${REMOTE_DIR}
            rm -rf ${REMOTE_HOME}/.deployment
            echo 'Done'
        " 2>&1
    fi
    info "Aether removed from ${HOST}"
    echo ""
    exit 0
fi

if $QUICK_MODE; then TOTAL_STEPS=5; else TOTAL_STEPS=6; fi

echo ""
echo "  ╔══════════════════════════════════════════════════════╗"
echo "  ║     🚀 Aether Kubernetes Deployment                ║"
echo "  ║     Universal Runtime Control Plane                  ║"
echo "  ╚══════════════════════════════════════════════════════╝"
echo ""
echo "  Host:      ${USER}@${HOST}"
echo "  Auth:      $([ -n "$PASS" ] && echo "🔑 password" || echo "🔐 SSH key")"
echo "  Cluster:   ${K8S_TYPE} (runtime: ${CRI}, build: ${BUILD_TOOL})"
echo "  Namespace: ${AETHER_NS}"
echo "  Image:     ${AETHER_IMAGE}"
echo "  NodePort:  ${NODE_PORT}"
echo "  Mode:      $(if $QUICK_MODE; then echo "⚡ quick (rebuild + redeploy)"; else echo "📦 full (build + deploy)"; fi)"
echo ""

# ── Step 1: Rsync repo ──
step "Step 1/${TOTAL_STEPS}: 📤 Syncing source to ${HOST}:~/.deployment/aether"
_ssh "mkdir -p ${REMOTE_DIR}" 2>&1
_rsync "$REPO_DIR/" "${USER}@${HOST}:${REMOTE_DIR}/" 2>&1 | tail -3
info "Synced to ${HOST}:${REMOTE_DIR}"

if ! $QUICK_MODE; then
    # ── Step 2: Install Rust toolchain ──
    step "Step 2/${TOTAL_STEPS}: 🦀 Ensuring Rust toolchain"
    _ssh "
        if command -v rustc &>/dev/null; then
            echo \"Rust: \$(rustc --version)\"
        else
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>&1 | tail -3
            source ${REMOTE_HOME}/.cargo/env
            echo \"Installed: \$(rustc --version)\"
        fi
    " 2>&1
    info "Rust toolchain ready"
fi

# ── Step N: Build container image ──
STEP=$(if $QUICK_MODE; then echo 2; else echo 3; fi)
step "Step ${STEP}/${TOTAL_STEPS}: 🐳 Building container image (${BUILD_TOOL})"
_ssh "
    source ${REMOTE_HOME}/.cargo/env 2>/dev/null || true
    cd ${REMOTE_DIR}

    # Build the release binary first (faster than docker multi-stage on small VPS)
    echo 'Building release binary...'
    cargo build --release 2>&1 | tail -3

    # Build container image with TLS support
    # Write Dockerfile using base64 to avoid SSH quote-stripping issues
    echo 'RlJPTSBkZWJpYW46Ym9va3dvcm0tc2xpbQpSVU4gYXB0LWdldCB1cGRhdGUgJiYgYXB0LWdldCBpbnN0YWxsIC15IGNhLWNlcnRpZmljYXRlcyBjdXJsIG9wZW5zc2wgJiYgcm0gLXJmIC92YXIvbGliL2FwdC9saXN0cy8qCkNPUFkgdGFyZ2V0L3JlbGVhc2UvYWV0aGVyIC91c3IvbG9jYWwvYmluL2FldGhlcgpSVU4gbWtkaXIgLXAgL3Zhci9saWIvYWV0aGVyIC90bHMKRVhQT1NFIDUwOTAKRU5UUllQT0lOVCBbIi91c3IvbG9jYWwvYmluL2FldGhlciJdCkNNRCBbInNlcnZlIiwgIi0taG9zdD0wLjAuMC4wIiwgIi0tcG9ydD01MDkwIl0K' | base64 -d > /tmp/Dockerfile.aether-deploy

    # Force rebuild (no cache) to pick up new binary
    ${BUILD_TOOL} build --no-cache -t ${AETHER_IMAGE} -f /tmp/Dockerfile.aether-deploy . 2>&1 | tail -5
    echo \"Image: \$(${BUILD_TOOL} images ${AETHER_IMAGE} --format '{{.Size}}')\"
" 2>&1
info "Container image built (${BUILD_TOOL})"

# ── Step N: Import image into cluster runtime ──
STEP=$(if $QUICK_MODE; then echo 3; else echo 4; fi)
step "Step ${STEP}/${TOTAL_STEPS}: 📦 Importing image into ${K8S_TYPE} (${CRI})"

if [ "$K8S_TYPE" = "k3s" ]; then
    # k3s uses its own bundled containerd
    _ssh "
        # Remove old image
        sudo /usr/local/bin/k3s ctr -n k8s.io images rm ${AETHER_IMAGE} 2>/dev/null || true

        # Import new image
        ${BUILD_TOOL} save ${AETHER_IMAGE} | sudo /usr/local/bin/k3s ctr images import - 2>&1 | tail -3
        echo 'Image imported into k3s containerd'
        sudo /usr/local/bin/k3s ctr images list | grep aether | head -3

        # Clean up build layers
        ${BUILD_TOOL} image prune -f 2>/dev/null || true
    " 2>&1
elif [ "$CRI" = "containerd" ]; then
    # Standard k8s with containerd
    _ssh "
        # Remove old image
        sudo ctr -n k8s.io images rm ${AETHER_IMAGE} 2>/dev/null || true

        # Import new image
        ${BUILD_TOOL} save ${AETHER_IMAGE} | sudo ctr -n k8s.io images import - 2>&1 | tail -3
        echo 'Image imported into containerd'
        sudo ctr -n k8s.io images list | grep aether | head -3

        # Clean up build layers
        ${BUILD_TOOL} image prune -f 2>/dev/null || true
    " 2>&1
elif [ "$CRI" = "docker" ]; then
    # Standard k8s with docker
    if [ "$BUILD_TOOL" = "docker" ]; then
        # Already in docker, nothing to import
        info "Image already in docker (no import needed)"
    else
        # Built with podman, load into docker
        _ssh "
            ${BUILD_TOOL} save ${AETHER_IMAGE} | sudo docker load 2>&1 | tail -3
            echo 'Image imported into docker'
            sudo docker images | grep aether | head -3

            # Clean up build layers
            ${BUILD_TOOL} image prune -f 2>/dev/null || true
        " 2>&1
    fi
else
    error "Unsupported container runtime: ${CRI}"
fi

info "Image available in ${K8S_TYPE} (${CRI})"

# ── Step N: Generate TLS cert + Apply K8s manifests ──
STEP=$(if $QUICK_MODE; then echo 4; else echo 5; fi)
step "Step ${STEP}/${TOTAL_STEPS}: ☸️  Deploying to Kubernetes (HTTPS)"
_ssh "
    # Create namespace
    kubectl create namespace ${AETHER_NS} --dry-run=client -o yaml | kubectl apply -f -

    # Generate self-signed TLS cert if the secret doesn't exist
    if ! kubectl -n ${AETHER_NS} get secret aether-tls &>/dev/null; then
        echo 'Generating TLS certificate for ${HOST}...'
        openssl req -x509 -nodes -days 365 \
            -newkey rsa:2048 \
            -keyout /tmp/aether-tls.key \
            -out /tmp/aether-tls.crt \
            -subj '/CN=${HOST}/O=Aether' \
            -addext 'subjectAltName=IP:${HOST}' 2>/dev/null
        kubectl -n ${AETHER_NS} create secret tls aether-tls \
            --cert=/tmp/aether-tls.crt \
            --key=/tmp/aether-tls.key
        rm -f /tmp/aether-tls.key /tmp/aether-tls.crt
        echo 'TLS secret created'
    else
        echo 'TLS secret already exists'
    fi

    # Apply RBAC, deployment + service with TLS
    kubectl apply -f - << 'K8SEOF'
---
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
- apiGroups: ["apps"]
  resources: ["deployments"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: [""]
  resources: ["pods", "pods/log", "services", "configmaps", "secrets", "nodes", "persistentvolumeclaims", "namespaces"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["batch"]
  resources: ["cronjobs", "jobs"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["networking.k8s.io"]
  resources: ["ingresses", "networkpolicies"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["autoscaling"]
  resources: ["horizontalpodautoscalers"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["kubevirt.io"]
  resources: ["virtualmachines", "virtualmachineinstances"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["cdi.kubevirt.io"]
  resources: ["datavolumes"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["subresources.kubevirt.io"]
  resources: ["virtualmachineinstances/console", "virtualmachineinstances/vnc"]
  verbs: ["get"]
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
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aether
  namespace: ${AETHER_NS}
  labels:
    app: aether
    app.kubernetes.io/name: aether
spec:
  replicas: 1
  selector:
    matchLabels:
      app: aether
  template:
    metadata:
      labels:
        app: aether
    spec:
      serviceAccountName: aether
      containers:
      - name: aether
        image: ${AETHER_IMAGE}
        imagePullPolicy: Never
        args:
        - serve
        - --host=0.0.0.0
        - --port=5090
        - --tls-cert=/tls/tls.crt
        - --tls-key=/tls/tls.key
        ports:
        - containerPort: 5090
          name: https
        env:
        - name: AETHER_LOG_FORMAT
          value: json
        - name: RUST_LOG
          value: info
        volumeMounts:
        - name: tls
          mountPath: /tls
          readOnly: true
        resources:
          requests:
            cpu: 100m
            memory: 64Mi
          limits:
            cpu: '1'
            memory: 256Mi
        livenessProbe:
          httpGet:
            path: /health
            port: 5090
            scheme: HTTPS
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 5090
            scheme: HTTPS
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: tls
        secret:
          secretName: aether-tls
---
apiVersion: v1
kind: Service
metadata:
  name: aether
  namespace: ${AETHER_NS}
  labels:
    app: aether
spec:
  type: NodePort
  selector:
    app: aether
  ports:
  - port: 5090
    targetPort: 5090
    nodePort: ${NODE_PORT}
    protocol: TCP
    name: https
K8SEOF

    # Force rollout to pick up the new image (tag is always :latest)
    echo 'Restarting deployment to use new image...'
    kubectl -n ${AETHER_NS} rollout restart deployment/aether
    kubectl -n ${AETHER_NS} rollout status deployment/aether --timeout=120s 2>&1

    # Remove old terminated pods
    kubectl -n ${AETHER_NS} delete pod --field-selector=status.phase==Succeeded 2>/dev/null || true

    echo ''
    echo 'Pod status:'
    kubectl -n ${AETHER_NS} get pods -o wide
" 2>&1
info "Kubernetes deployment ready (HTTPS)"

# Open NodePort in firewall
_ssh "
    if ! $SUDO iptables -C INPUT -p tcp --dport ${NODE_PORT} -j ACCEPT 2>/dev/null; then
        $SUDO iptables -I INPUT -p tcp --dport ${NODE_PORT} -j ACCEPT
        echo 'Firewall: port ${NODE_PORT}/tcp opened'
    else
        echo 'Firewall: port ${NODE_PORT}/tcp already open'
    fi
" 2>&1

# ── Step N: Clean up source ──
STEP=$(if $QUICK_MODE; then echo 5; else echo 6; fi)
step "Step ${STEP}/${TOTAL_STEPS}: 🧹 Cleaning up source code"
_ssh "
    rm -rf ${REMOTE_DIR}
    echo 'Source cleaned from ${REMOTE_DIR}'
" 2>&1
info "Source cleaned"

# ── Verify ──
step "Verifying deployment"
_ssh "
    echo '📍 Namespace:'
    kubectl -n ${AETHER_NS} get all 2>&1

    echo ''
    echo '📍 Pod logs (last 5 lines):'
    POD=\$(kubectl -n ${AETHER_NS} get pods -l app=aether -o jsonpath='{.items[0].metadata.name}' 2>/dev/null)
    kubectl -n ${AETHER_NS} logs \$POD --tail=5 2>/dev/null || echo '  (no logs yet)'
" 2>&1

# ── Smoke test ──
step "Smoke testing endpoints"
_ssh "
    pass=0; fail=0
    test_ep() {
        local label=\"\$1\" url=\"\$2\" expect=\"\$3\"
        code=\$(curl -sk -o /dev/null -w '%{http_code}' --max-time 5 \"\$url\" 2>/dev/null)
        if [ \"\$code\" = \"\$expect\" ]; then
            printf '  ✅ %-20s %s -> %s\n' \"\$label\" \"\$url\" \"\$code\"
            pass=\$((pass + 1))
        else
            printf '  ❌ %-20s %s -> %s (expected %s)\n' \"\$label\" \"\$url\" \"\$code\" \"\$expect\"
            fail=\$((fail + 1))
        fi
    }

    echo ''
    test_ep 'Dashboard'   'https://localhost:${NODE_PORT}/'               200
    test_ep 'Health'      'https://localhost:${NODE_PORT}/health'          200
    test_ep 'API'         'https://localhost:${NODE_PORT}/api/workloads'   200
    test_ep 'Metrics'     'https://localhost:${NODE_PORT}/api/metrics'     200
    echo ''
    echo \"  Results: \${pass} passed, \${fail} failed\"
" 2>&1

echo ""
echo "  ════════════════════════════════════════════════════════"
echo "  🎉 Deployment complete!"
echo "  ════════════════════════════════════════════════════════"
echo ""
echo "  🔗 SSH Access:"
echo "    Host:     ${HOST}"
echo "    User:     ${USER}"
echo "    Password: ${PASS}"
echo "    Command:  ssh ${USER}@${HOST}"
echo ""
echo "  🌐 Web Dashboard:"
echo "    URL:      https://${HOST}:${NODE_PORT}"
echo ""
echo "  📡 API Endpoints:"
echo "    Health:   https://${HOST}:${NODE_PORT}/health"
echo "    API:      https://${HOST}:${NODE_PORT}/api/workloads"
echo "    Metrics:  https://${HOST}:${NODE_PORT}/api/metrics"
echo "    SSE:      https://${HOST}:${NODE_PORT}/api/events/stream"
echo ""
echo "  ☸️  Kubernetes (${K8S_TYPE}, ${CRI}):"
echo "    Namespace:  ${AETHER_NS}"
echo "    Deployment: aether"
echo "    Service:    aether (NodePort ${NODE_PORT})"
echo "    kubectl -n ${AETHER_NS} get pods"
echo "    kubectl -n ${AETHER_NS} logs -f deployment/aether"
echo ""
echo "  🔄 Redeploy:"
echo "    $0 ${HOST} ${USER} --quick"
echo ""
