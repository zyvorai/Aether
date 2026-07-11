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
#
# Environment:
#   AETHER_SKIP_CILIUM_BOOTSTRAP=1 — skip Cilium bootstrap manifests
#   AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP=1 — legacy alias for the above
#   AETHER_CILIUM_EGRESS_STRICT=1 — Cilium: kube-apiserver + DNS only (see deploy/k8s/bootstrap/)
#   AETHER_CILIUM_STRICT_ALLOW_CLUSTER=1 — with strict, also allow in-cluster pod/service traffic
#   AETHER_INSTALL_METRICS_SERVER=auto|1|0 — install metrics-server when missing (auto on k3s/kind)
#   AETHER_SKIP_CILIUM_CONNECTIVITY=1 — skip Cilium agent probe and CronJob
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"
# shellcheck source=lib-deploy-cluster.sh
source "${SCRIPT_DIR}/lib-deploy-cluster.sh"
# shellcheck source=lib-deploy-manifest.sh
source "${SCRIPT_DIR}/lib-deploy-manifest.sh"

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
  local pull_policy
  pull_policy="$(aether_deploy_image_pull_policy "${IMAGE_LATEST}")"
  detect_k8s_distro
  aether_install_metrics_server "${KUBECTL}" "${DISTRO}"
  aether_deploy_build_secret_env_blocks "${NAMESPACE}"
  local svc_ingress
  svc_ingress="$(aether_deploy_service_ingress_yaml "${NAMESPACE}" "${AETHER_EXPOSE:-nodeport}" "${NODE_PORT}")"

  ${KUBECTL} create namespace "${NAMESPACE}" --dry-run=client -o yaml | ${KUBECTL} apply -f -
  aether_apply_rbac "${REPO_ROOT}" "${NAMESPACE}" "${KUBECTL}"
  aether_apply_cilium_bootstrap "${REPO_ROOT}" "${NAMESPACE}" "${KUBECTL}"
  aether_probe_cilium_connectivity "${NAMESPACE}" "${KUBECTL}" || true
  aether_apply_cilium_connectivity_cronjob "${REPO_ROOT}" "${NAMESPACE}" "${KUBECTL}"
  {
    printf '%s\n' "${AETHER_MANIFEST_SECRETS_YAML}"
    cat <<EOF
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
        imagePullPolicy: ${pull_policy}
        args: ['serve', '--host=0.0.0.0', '--port=5090']
        ports:
        - containerPort: 5090
          name: http
        env:
        - name: AETHER_LOG_FORMAT
          value: json
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
${svc_ingress}
EOF
  } | ${KUBECTL} apply -f -
}

cmd_detect() {
  detect_container_runtime
  detect_k8s_distro
  echo ""
  aether_sparkle_line "Local cluster"
  echo -e "${A_CYN}${A_BLD}     🔭  Environment probe${A_RST}"
  echo ""
  aether_kv "Container runtime" "${RUNTIME}"
  aether_kv "Kubectl" "${KUBECTL}"
  aether_kv "K8s distro" "${DISTRO}"
  aether_kv "Namespace" "${NAMESPACE}"
  aether_kv "Image" "${IMAGE_LATEST}"
  if ${KUBECTL} cluster-info &>/dev/null; then
    aether_print_cluster_mesh_report "${KUBECTL}"
  else
    aether_subtle "     (cluster unreachable — skipping mesh probe)"
    echo ""
  fi
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
  aether_delete_managed_cilium_policies "${NAMESPACE}" "${KUBECTL}"
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
    echo ""
    aether_ok "🪄  Build → load → deploy chain complete (local cluster)"
    aether_subtle "     Tip: ./scripts/health-check-all.sh for the full ritual."
    ;;
  *)
    aether_die "Unknown command: ${COMMAND}"
    ;;
esac
