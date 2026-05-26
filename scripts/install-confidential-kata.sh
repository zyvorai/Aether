#!/usr/bin/env bash
# ============================================================================
# install-confidential-kata.sh — Helm install Confidential Containers + Zyvor RuntimeClasses
#
# Installs:
#   1. Confidential Containers (official CoCo chart) — kata shims on nodes
#   2. Zyvor RuntimeClasses (charts/zyvor-confidential-kata) — kata-clh-* names for fabric
#
# Usage:
#   ./scripts/install-confidential-kata.sh
#   ./scripts/install-confidential-kata.sh --dry-run
#   ./scripts/install-confidential-kata.sh --runtime-classes-only
#   ./scripts/install-confidential-kata.sh --gpu
#   K8S_DISTRIBUTION=k3s ./scripts/install-confidential-kata.sh
#
# Environment:
#   COCO_RELEASE          Helm release name (default coco)
#   COCO_NAMESPACE        Namespace (default coco-system)
#   ZYVOR_RC_RELEASE      Zyvor RuntimeClass release (default zyvor-coco)
#   ZYVOR_RC_NAMESPACE    Namespace for Zyvor chart (default default)
#   K8S_DISTRIBUTION      k8s | k3s | rke2 | k0s | microk8s (auto-detected if unset)
#   COCO_CHART_VERSION    Pin CoCo chart version (optional)
#   HELM_TIMEOUT          Per-release timeout (default 15m)
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Script may live at repo/scripts/ or bundle root (customer tarball).
CHART_DIR=""
for candidate in \
  "${SCRIPT_DIR}/charts/zyvor-confidential-kata" \
  "${SCRIPT_DIR}/../charts/zyvor-confidential-kata"; do
  if [ -f "${candidate}/Chart.yaml" ]; then
    CHART_DIR="${candidate}"
    REPO_ROOT="$(cd "$(dirname "${candidate}")/.." && pwd)"
    break
  fi
done

COCO_RELEASE="${COCO_RELEASE:-coco}"
COCO_NAMESPACE="${COCO_NAMESPACE:-coco-system}"
ZYVOR_RC_RELEASE="${ZYVOR_RC_RELEASE:-zyvor-coco}"
ZYVOR_RC_NAMESPACE="${ZYVOR_RC_NAMESPACE:-default}"
HELM_TIMEOUT="${HELM_TIMEOUT:-15m}"
COCO_CHART="oci://ghcr.io/confidential-containers/charts/confidential-containers"

DRY_RUN=false
RUNTIME_CLASSES_ONLY=false
ENABLE_GPU=false

usage() {
  cat <<'EOF'
Usage: install-confidential-kata.sh [options]

Options:
  --dry-run                 Print helm commands without applying
  --runtime-classes-only    Skip CoCo install; apply Zyvor RuntimeClasses only
  --gpu                     Enable kata-clh-gpu-snp RuntimeClass (values)
  -h, --help                Show this help

Requires: kubectl, helm 3.8+
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=true ;;
    --runtime-classes-only) RUNTIME_CLASSES_ONLY=true ;;
    --gpu) ENABLE_GPU=true ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
  shift
done

log() { echo "[confidential-kata] $*"; }
warn() { echo "[confidential-kata] WARN: $*" >&2; }
die() { echo "[confidential-kata] ERROR: $*" >&2; exit 1; }

command -v kubectl >/dev/null 2>&1 || die "kubectl not found"
command -v helm >/dev/null 2>&1 || die "helm not found (Helm 3.8+ required)"
[ -n "${CHART_DIR}" ] && [ -d "${CHART_DIR}" ] || die "missing charts/zyvor-confidential-kata (run from repo root or bundle extract)"

detect_k8s_distribution() {
  if [ -n "${K8S_DISTRIBUTION:-}" ]; then
    echo "${K8S_DISTRIBUTION}"
    return
  fi
  local ver labels
  ver="$(kubectl get nodes -o jsonpath='{.items[0].status.nodeInfo.kubeletVersion}' 2>/dev/null || true)"
  labels="$(kubectl get nodes -o jsonpath='{.items[0].metadata.labels}' 2>/dev/null || true)"
  if echo "${ver}" | grep -qi 'k3s'; then
    echo k3s
  elif echo "${labels}" | grep -qi 'rke2'; then
    echo rke2
  elif echo "${labels}" | grep -qi 'k0s'; then
    echo k0s
  elif echo "${labels}" | grep -qi 'microk8s'; then
    echo microk8s
  else
    echo k8s
  fi
}

run_helm() {
  if $DRY_RUN; then
    echo "[dry-run] helm $*"
    return 0
  fi
  helm "$@"
}

install_coco() {
  local distro="$1"
  log "Installing Confidential Containers (release=${COCO_RELEASE}, ns=${COCO_NAMESPACE}, distro=${distro})"
  local args=(
    upgrade --install "${COCO_RELEASE}" "${COCO_CHART}"
    --namespace "${COCO_NAMESPACE}"
    --create-namespace
    --timeout "${HELM_TIMEOUT}"
    --wait
  )
  if [ -n "${COCO_CHART_VERSION:-}" ]; then
    args+=(--version "${COCO_CHART_VERSION}")
  fi
  if [ "${distro}" != "k8s" ]; then
    args+=(--set "kata-as-coco-runtime.k8sDistribution=${distro}")
  fi
  run_helm "${args[@]}"
  if ! $DRY_RUN; then
    log "Waiting for CoCo kata-deploy daemonset..."
    kubectl -n "${COCO_NAMESPACE}" rollout status daemonset/kata-deploy --timeout="${HELM_TIMEOUT}" 2>/dev/null \
      || kubectl -n "${COCO_NAMESPACE}" wait --for=condition=Ready pod -l name=kata-deploy --timeout="${HELM_TIMEOUT}" 2>/dev/null \
      || warn "Could not confirm kata-deploy readiness — check: kubectl get pods -n ${COCO_NAMESPACE}"
  fi
}

install_zyvor_runtime_classes() {
  log "Installing Zyvor RuntimeClasses (release=${ZYVOR_RC_RELEASE}, ns=${ZYVOR_RC_NAMESPACE})"
  local values_args=()
  if $ENABLE_GPU; then
    values_args+=(--set runtimeClasses.clhGpuSnp.enabled=true)
  fi
  run_helm upgrade --install "${ZYVOR_RC_RELEASE}" "${CHART_DIR}" \
    --namespace "${ZYVOR_RC_NAMESPACE}" \
    --timeout 5m \
    "${values_args[@]}"
}

verify_install() {
  if $DRY_RUN; then
    return 0
  fi
  log "RuntimeClasses:"
  kubectl get runtimeclass 2>/dev/null | grep -E 'kata-clh|kata-qemu|NAME' || warn "No kata runtime classes listed yet"
  log "CoCo pods:"
  kubectl get pods -n "${COCO_NAMESPACE}" 2>/dev/null | head -10 || true
}

main() {
  local distro
  distro="$(detect_k8s_distribution)"
  log "Detected Kubernetes distribution: ${distro}"

  if ! $RUNTIME_CLASSES_ONLY; then
    install_coco "${distro}"
  else
    log "Skipping CoCo install (--runtime-classes-only)"
  fi

  install_zyvor_runtime_classes
  verify_install
  log "Done. Label TEE nodes (example): kubectl label node <node> node.kubernetes.io/sev-snp=true"
  log "Docs: docs/CONFIDENTIAL.md · deploy/confidential/"
}

main "$@"
