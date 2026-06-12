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
# Only two positional arguments are used: <host> and optional [user]. Do not pass a URL
# as a third argument (it is ignored); open TCP on AETHER_NODE_PORT (default 30090) in the
# host firewall and cloud security group if the service is not reachable from the internet.
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
#   AETHER_DEPLOY_REPLICAS — API Deployment replicas (default 1). Use 2+ only with shared RWX state or read-only replicas; local JSON state is single-writer.
#   AETHER_DEPLOY_PDB — when AETHER_DEPLOY_REPLICAS>=2, apply a PodDisruptionBudget (default 1). Set 0 to skip.
#   AETHER_SKIP_CILIUM_BOOTSTRAP=1 — skip Cilium bootstrap (same as legacy AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP)
#   AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP=1 — legacy alias for the above
#   AETHER_CILIUM_EGRESS_STRICT=1 — Cilium: kube-apiserver + DNS only (instead of toEntities: all)
#   AETHER_CILIUM_STRICT_ALLOW_CLUSTER=1 — with strict, also allow toEntities: cluster (in-cluster workloads)
#   AETHER_SKIP_EXTERNAL_HEALTH=1 — skip curl from this machine to http://<host>:<nodePort>/health (useful in CI)
#   AETHER_EXPOSE — nodeport | ingress | both (default nodeport)
#   AETHER_INGRESS_HOST — hostname for Ingress (required when EXPOSE includes ingress)
#   AETHER_INGRESS_CLASS — ingressClassName (e.g. traefik, nginx)
#   AETHER_INGRESS_TLS_SECRET — TLS secret for Ingress (or set AETHER_INGRESS_TLS_ACME=1 for cert-manager)
#   AETHER_INGRESS_TLS_ACME=1 — annotate Ingress for cert-manager (issuer: AETHER_INGRESS_ACME_ISSUER)
#   AETHER_REGISTRY — image repo (default localhost/aether); set ghcr.io/user/aether with AETHER_PUSH_IMAGE=1
#   AETHER_PUSH_IMAGE=1 — docker/podman push after build (non-localhost images)
#   AETHER_STATE_DATABASE_URL — PostgreSQL URI for HA API replicas
#   AETHER_REDIS_URL — Redis for OIDC sessions across replicas
#   AETHER_OIDC_* — OIDC issuer, client, redirect, session secret (see src/oidc.rs)
#   AETHER_BACKUP_REMOTE_URL / AETHER_BACKUP_REMOTE_TOKEN — HTTP PUT backup offload
#   AETHER_AUDIT_WEBHOOK_URL — POST audit events to external sink
#   AETHER_OPEN_FIREWALL=1 — best-effort ufw allow on remote (ports 80/443 and/or NodePort)
#
# Reachability: SSH keys only affect ssh/rsync. For NodePort, open TCP AETHER_NODE_PORT in cloud firewall.
# Prefer AETHER_EXPOSE=ingress with AETHER_INGRESS_HOST for HTTPS on 443.
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"
# shellcheck source=lib-deploy-manifest.sh
source "${SCRIPT_DIR}/lib-deploy-manifest.sh"

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
      sed -n '2,39p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) POSITIONAL+=("$arg") ;;
  esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-root}}"
PASS="${DEPLOY_PASS:-}"

if [ "${#POSITIONAL[@]}" -gt 2 ]; then
  aether_warn "Ignoring extra argument(s) after <host> [user]: only '${HOST}' and '${USER}' are used (${#POSITIONAL[@]} args given)."
fi

[ -n "${HOST}" ] || {
  echo "Usage: $0 [flags] <host> [user]"
  exit 1
}

AETHER_NS="${AETHER_NAMESPACE:-aether-system}"
AETHER_REGISTRY="${AETHER_REGISTRY:-localhost/aether}"
AETHER_IMAGE="${AETHER_IMAGE:-${AETHER_REGISTRY}:latest}"
NODE_PORT="${AETHER_NODE_PORT:-30090}"
AETHER_EXPOSE="${AETHER_EXPOSE:-nodeport}"
AETHER_INGRESS_HOST="${AETHER_INGRESS_HOST:-}"
IMAGE_PULL_POLICY="$(aether_deploy_image_pull_policy "${AETHER_IMAGE}")"
DEPLOY_REPLICAS="${AETHER_DEPLOY_REPLICAS:-1}"
DEPLOY_PDB="${AETHER_DEPLOY_PDB:-1}"
case "${DEPLOY_REPLICAS}" in
  '' | *[!0-9]*) DEPLOY_REPLICAS=1 ;;
esac
if [ "${DEPLOY_REPLICAS}" -lt 1 ]; then
  DEPLOY_REPLICAS=1
fi
PDB_YAML_APPEND=""
if [ "${DEPLOY_REPLICAS}" -ge 2 ] && [ "${DEPLOY_PDB}" != "0" ]; then
  PDB_YAML_APPEND="$(printf '%s\n' "---
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: aether
  namespace: ${AETHER_NS}
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app: aether")"
fi
AETHER_API_KEY="${AETHER_API_KEY:-}"
AETHER_LOG_FORMAT="${AETHER_LOG_FORMAT:-json}"
SKIP_EXT_HEALTH="${AETHER_SKIP_EXTERNAL_HEALTH:-0}"

info() { aether_ok "$*"; }
warn() { aether_warn "$*"; }
step() { aether_step_remote "$@"; }
error() { aether_die "$*"; }

DEPLOY_START=$(date +%s)

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

# Run a multi-line bash script on the remote (stdin). Expands locals in the caller's heredoc.
remote_bash() {
  if [ -n "${PASS}" ]; then
    SSHPASS="${PASS}" sshpass -e ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" bash -s
  else
    ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" bash -s
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

aether_banner_remote
aether_kv_icon "🖥️" "Host" "${USER}@${HOST}"
aether_kv_icon "☸️" "Cluster" "${REMOTE_K8S_FLAVOR}"
aether_kv_icon "🔧" "Kubectl" "${K}"
aether_kv_icon "🐳" "Builder" "${BUILD_TOOL}"
aether_kv_icon "📦" "Import" "${IMPORT_CMD}"
aether_kv_icon "📁" "Namespace" "${AETHER_NS}"
aether_kv_icon "🏷️" "Image" "${AETHER_IMAGE}"
aether_kv_icon "🌐" "Expose" "${AETHER_EXPOSE}"
aether_kv_icon "🔌" "NodePort" "${NODE_PORT}"
aether_kv_icon "🌍" "Ingress" "${AETHER_INGRESS_HOST:-—}"
aether_kv_icon "📊" "Replicas" "${DEPLOY_REPLICAS}"
aether_kv_icon "⬇️" "Pull policy" "${IMAGE_PULL_POLICY}"
FAST_FLAGS=""
[ "${SKIP_RSYNC}" = "1" ] && FAST_FLAGS="${FAST_FLAGS} skip-sync"
[ "${SKIP_CARGO}" = "1" ] && FAST_FLAGS="${FAST_FLAGS} skip-cargo"
[ "${SKIP_IMAGE}" = "1" ] && FAST_FLAGS="${FAST_FLAGS} skip-image"
[ "${LOCAL_BUILD}" = "1" ] && FAST_FLAGS="${FAST_FLAGS} local-build"
aether_kv_icon "⚡" "Fast path" "${FAST_FLAGS:-full deploy}"
echo ""

if [ "${DEPLOY_REPLICAS}" -ge 2 ] && [ -z "${AETHER_STATE_DATABASE_URL:-}" ]; then
  warn "AETHER_DEPLOY_REPLICAS>=2 without AETHER_STATE_DATABASE_URL — each pod has isolated JSON state. Set Postgres URL or use replicas=1."
fi
if { [ "${AETHER_EXPOSE}" = "ingress" ] || [ "${AETHER_EXPOSE}" = "both" ]; } && [ -z "${AETHER_INGRESS_HOST}" ]; then
  error "AETHER_INGRESS_HOST is required when AETHER_EXPOSE=${AETHER_EXPOSE}"
fi

if [ "${UNINSTALL}" = "1" ]; then
  step 1 1 "🗑️" "Removing Aether from remote cluster"
  remote_bash <<UNINSTALL
set -euo pipefail
source "${REMOTE_DIR}/scripts/lib-deploy-pretty.sh"
source "${REMOTE_DIR}/scripts/lib-deploy-cluster.sh"
aether_delete_managed_cilium_policies "${AETHER_NS}" "${K}"
${K} delete namespace ${AETHER_NS} --ignore-not-found
${K} delete clusterrole aether-discovery --ignore-not-found
${K} delete clusterrolebinding aether-discovery --ignore-not-found
rm -rf ${REMOTE_DIR}
UNINSTALL
  info "Aether removed from ${HOST}"
  aether_finale_uninstall
  exit 0
fi

TOTAL_STEPS=5

step 1 "${TOTAL_STEPS}" "📡" "Syncing source"
if [ "${SKIP_RSYNC}" = "1" ]; then
  info "Skipped rsync — using existing tree on remote"
else
  ssh_cmd "mkdir -p ${REMOTE_DIR}"
  rsync_cmd "${REPO_ROOT}/" "${USER}@${HOST}:${REMOTE_DIR}/"
  info "Source synced to ${REMOTE_DIR}"
fi

step 2 "${TOTAL_STEPS}" "🦀" "Building dashboard + release binary"
if [ "${SKIP_RSYNC}" != "1" ] || [ -d "${REPO_ROOT}/web/dashboard/node_modules" ]; then
  if [ -f "${REPO_ROOT}/web/dashboard/package.json" ]; then
    info "Building embedded dashboard (web/dashboard/dist)..."
    (cd "${REPO_ROOT}/web/dashboard" && (npm ci --prefer-offline 2>/dev/null || npm install) && npm run test && npm run validate:schema && npm run build)
  fi
elif [ -f "${REPO_ROOT}/web/dashboard/dist/index.html" ]; then
  info "Using existing web/dashboard/dist (set AETHER_SKIP_RSYNC=0 and sync source to rebuild UI)"
else
  warn "No web/dashboard/dist — run: cd web/dashboard && npm install && npm run build"
fi

if [ "${LOCAL_BUILD}" = "1" ]; then
  REMOTE_UNAME="$(ssh_cmd 'uname -srm' 2>/dev/null || true)"
  LOCAL_BIN="${REPO_ROOT}/target/release/aether"
  LOCAL_FILE="$(file -b "${LOCAL_BIN}" 2>/dev/null || true)"
  if [ -n "${REMOTE_UNAME}" ] && echo "${REMOTE_UNAME}" | grep -qi 'linux'; then
    if echo "${LOCAL_FILE}" | grep -qiE 'Mach-O|darwin'; then
      if [ "${AETHER_LOCAL_BUILD_FORCE:-0}" != "1" ]; then
        error "--local-build refused: local binary is macOS (${LOCAL_FILE}) but remote is Linux (${REMOTE_UNAME}). Use remote cargo (default) or set AETHER_LOCAL_BUILD_FORCE=1 to override."
      fi
      warn "AETHER_LOCAL_BUILD_FORCE=1: uploading macOS binary to Linux remote — pod will crash unless cross-compiled."
    fi
  fi
  (cd "${REPO_ROOT}" && cargo build --release)
  strip "${LOCAL_BIN}" 2>/dev/null || true
  ssh_cmd "mkdir -p ${REMOTE_DIR}/target/release"
  rsync_cmd "${LOCAL_BIN}" "${USER}@${HOST}:${REMOTE_DIR}/target/release/aether"
  info "Binary built locally and uploaded"
  if echo "${LOCAL_FILE}" | grep -qiE 'Mach-O|darwin'; then
    aether_hint "Local build uploaded a macOS binary — use remote cargo on Linux nodes unless cross-compiling."
  fi
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
    if [ -f web/dashboard/package.json ]; then
      cd web/dashboard && (npm ci --prefer-offline 2>/dev/null || npm install) && npm run test && npm run validate:schema && npm run build
    fi
    cargo build --release
    strip target/release/aether 2>/dev/null || true
  "
  info "Binary built on remote"
fi

step 3 "${TOTAL_STEPS}" "🐳" "Building and importing image"
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
    if [ \"${AETHER_PUSH_IMAGE:-0}\" = \"1\" ]; then
      ${BUILD_TOOL} push ${AETHER_IMAGE}
    fi
    case \"${AETHER_IMAGE}\" in
      localhost/*|127.0.0.1/*)
        ${BUILD_TOOL} save ${AETHER_IMAGE} | ${IMPORT_CMD}
        ;;
      *)
        echo \"Non-local image — skipping ctr import; cluster must pull ${AETHER_IMAGE}\"
        ;;
    esac
  "
  info "Image built and imported"
fi

step 4 "${TOTAL_STEPS}" "☸️" "Applying Kubernetes manifests"
DEPLOY_STAMP="$(date +%s)-${RANDOM}"
aether_deploy_build_secret_env_blocks "${AETHER_NS}"
SVC_INGRESS_YAML="$(aether_deploy_service_ingress_yaml "${AETHER_NS}" "${AETHER_EXPOSE}" "${NODE_PORT}")"

remote_bash <<REMOTE_APPLY
set -euo pipefail
${K} create namespace ${AETHER_NS} --dry-run=client -o yaml | ${K} apply -f -
source "${REMOTE_DIR}/scripts/lib-deploy-pretty.sh"
source "${REMOTE_DIR}/scripts/lib-deploy-cluster.sh"
source "${REMOTE_DIR}/scripts/lib-deploy-manifest.sh"
aether_apply_rbac "${REMOTE_DIR}" "${AETHER_NS}" "${K}"
aether_apply_cilium_bootstrap "${REMOTE_DIR}" "${AETHER_NS}" "${K}"
_DISTRO=k8s
if ${K} get nodes -o jsonpath='{.items[0].status.nodeInfo.kubeletVersion}' 2>/dev/null | grep -qi k3s; then _DISTRO=k3s; fi
aether_install_metrics_server "${K}" "\${_DISTRO}"
aether_probe_cilium_connectivity "${AETHER_NS}" "${K}" || true
aether_apply_cilium_connectivity_cronjob "${REMOTE_DIR}" "${AETHER_NS}" "${K}"
printf '%s' "${AETHER_MANIFEST_SECRETS_YAML}"
cat <<YAML | ${K} apply -f -
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aether
  namespace: ${AETHER_NS}
spec:
  replicas: ${DEPLOY_REPLICAS}
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
        imagePullPolicy: ${IMAGE_PULL_POLICY}
        args: ['serve', '--host=0.0.0.0', '--port=5090']
        ports:
        - containerPort: 5090
          name: http
        env:
        - name: AETHER_LOG_FORMAT
          value: '${AETHER_LOG_FORMAT}'
        - name: RUST_LOG
          value: info
${AETHER_MANIFEST_EXTRA_ENV_YAML}
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
${SVC_INGRESS_YAML}
${PDB_YAML_APPEND}
YAML
${K} -n ${AETHER_NS} rollout restart deployment/aether >/dev/null 2>&1 || true
${K} -n ${AETHER_NS} rollout status deployment/aether --timeout=180s
REMOTE_APPLY
aether_deploy_try_open_firewall ssh_cmd "${AETHER_EXPOSE}" "${NODE_PORT}"
info "Kubernetes deployment applied"

step 5 "${TOTAL_STEPS}" "🔍" "Verifying rollout & health"
ssh_cmd "
  ${K} -n ${AETHER_NS} get pods -o wide
  ${K} -n ${AETHER_NS} get svc aether
" || error "Verification failed"

LOCAL_HP="$(ssh_cmd "command -v curl >/dev/null 2>&1 && { code=\$(curl -sS -m 12 -o /dev/null -w '%{http_code}' http://127.0.0.1:${NODE_PORT}/health 2>/dev/null || true); echo \"\${code:-000}\"; } || echo nocurl" | tr -d '\r' | tail -1)"
if [ "${LOCAL_HP}" = "200" ]; then
  info "NodePort responds on the server (curl 127.0.0.1:${NODE_PORT}/health → 200)"
  if [ "${SKIP_EXT_HEALTH}" != "1" ] && command -v curl >/dev/null 2>&1; then
    EXT_HP="$(curl -sS -m 12 -o /dev/null -w '%{http_code}' "http://${HOST}:${NODE_PORT}/health" 2>/dev/null || printf '%s' "000")"
    if [ "${EXT_HP}" = "200" ]; then
      info "Health check from this machine → http://${HOST}:${NODE_PORT}/health (200)"
    else
      warn "Health check from this machine → http://${HOST}:${NODE_PORT}/health returned '${EXT_HP}' (not 200)."
      warn "SSH and the in-cluster deploy are unrelated to this: open inbound TCP ${NODE_PORT} on the server (sudo ufw allow ${NODE_PORT}/tcp) and in your cloud provider firewall / security group, then retry the URL."
    fi
  elif [ "${SKIP_EXT_HEALTH}" != "1" ]; then
    warn "Install curl on this machine to probe http://${HOST}:${NODE_PORT}/health from here, or test in a browser after opening TCP ${NODE_PORT} in the cloud firewall."
  fi
elif [ "${LOCAL_HP}" = "nocurl" ]; then
  warn "Remote host has no curl — install curl to auto-probe NodePort, or test manually: curl -sS http://127.0.0.1:${NODE_PORT}/health"
else
  warn "NodePort health from server loopback returned '${LOCAL_HP}' (expected 200)."
  warn "Pod diagnostics (describe + logs):"
  ssh_cmd "${K} -n ${AETHER_NS} get events --sort-by=.lastTimestamp | tail -n 25" || true
  ssh_cmd "${K} -n ${AETHER_NS} describe pod -l app=aether 2>/dev/null | tail -n 80" || true
  ssh_cmd "${K} -n ${AETHER_NS} logs deploy/aether --tail=60 2>&1" || true
  warn "Multi-node cluster? imagePullPolicy:Never requires the image on every node that can run the pod. Re-run image import on each node or use a registry."
fi

if [ -n "${AETHER_INGRESS_HOST}" ] && { [ "${AETHER_EXPOSE}" = "ingress" ] || [ "${AETHER_EXPOSE}" = "both" ]; } && [ "${SKIP_EXT_HEALTH}" != "1" ] && command -v curl >/dev/null 2>&1; then
  ING_SCHEME="http"
  if [ -n "${AETHER_INGRESS_TLS_SECRET:-}" ] || [ "${AETHER_INGRESS_TLS_ACME:-}" = "1" ]; then
    ING_SCHEME="https"
  fi
  ING_HP="$(curl -sS -m 12 -o /dev/null -w '%{http_code}' "${ING_SCHEME}://${AETHER_INGRESS_HOST}/health" 2>/dev/null || printf '%s' "000")"
  if [ "${ING_HP}" = "200" ]; then
    info "Ingress health → ${ING_SCHEME}://${AETHER_INGRESS_HOST}/health (200)"
  else
    warn "Ingress health → ${ING_SCHEME}://${AETHER_INGRESS_HOST}/health returned '${ING_HP}' (DNS must point at this cluster; Ingress controller required)."
  fi
fi

PUBLIC_URL="http://${HOST}:${NODE_PORT}"
HEALTH_URL="http://${HOST}:${NODE_PORT}/health"
if [ -n "${AETHER_INGRESS_HOST}" ] && { [ "${AETHER_EXPOSE}" = "ingress" ] || [ "${AETHER_EXPOSE}" = "both" ]; }; then
  if [ -n "${AETHER_INGRESS_TLS_SECRET:-}" ] || [ "${AETHER_INGRESS_TLS_ACME:-}" = "1" ]; then
    PUBLIC_URL="https://${AETHER_INGRESS_HOST}"
  else
    PUBLIC_URL="http://${AETHER_INGRESS_HOST}"
  fi
  HEALTH_URL="${PUBLIC_URL}/health"
fi
DEPLOY_ELAPSED=$(( $(date +%s) - DEPLOY_START ))
aether_finale_remote_deploy "${PUBLIC_URL}" "${HEALTH_URL}" "${DEPLOY_ELAPSED}"
if [ -n "${AETHER_API_KEY}" ]; then
  info "🔐 API authentication is enabled on the remote deployment"
fi

if [ "${AETHER_SKIP_POST_DEPLOY_VERIFY:-0}" != "1" ] && [ "${SKIP_EXT_HEALTH}" != "1" ]; then
  echo ""
  info "Running post-deploy API verification…"
  if AETHER_API="${HEALTH_URL%/health}" "${SCRIPT_DIR}/post-deploy-verify.sh"; then
    info "Post-deploy verification passed"
  else
    warn "Post-deploy verification had failures (deployment is live — check logs above)"
  fi
fi
