#!/usr/bin/env bash
# ============================================================================
# install-confidential-spire.sh — Helm install SPIRE for workload SPIFFE identity
#
# Usage:
#   ./scripts/install-confidential-spire.sh
#   ./scripts/install-confidential-spire.sh --dry-run
#
# Environment:
#   SPIRE_RELEASE       default spire
#   SPIRE_NAMESPACE     default spire
#   SPIFFE_HELM_REPO      default https://spiffe.github.io/helm-charts-hardened/
#   HELM_TIMEOUT          default 15m
# ============================================================================

set -euo pipefail

SPIRE_RELEASE="${SPIRE_RELEASE:-spire}"
SPIRE_NAMESPACE="${SPIRE_NAMESPACE:-spire}"
SPIFFE_HELM_REPO="${SPIFFE_HELM_REPO:-https://spiffe.github.io/helm-charts-hardened/}"
HELM_TIMEOUT="${HELM_TIMEOUT:-15m}"
DRY_RUN=false

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=true ;;
    -h|--help)
      echo "Usage: install-confidential-spire.sh [--dry-run]"
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
  shift
done

log() { echo "[confidential-spire] $*"; }
die() { echo "[confidential-spire] ERROR: $*" >&2; exit 1; }

command -v helm >/dev/null 2>&1 || die "helm not found"
command -v kubectl >/dev/null 2>&1 || die "kubectl not found"

run_helm() {
  if $DRY_RUN; then
    echo "[dry-run] helm $*"
  else
    helm "$@"
  fi
}

log "Adding SPIFFE Helm repo..."
if ! $DRY_RUN; then
  helm repo add spiffe "${SPIFFE_HELM_REPO}" 2>/dev/null || true
  helm repo update spiffe 2>/dev/null || helm repo update
fi

log "Installing SPIRE CRDs (spire-crds chart)..."
run_helm upgrade --install spire-crds spiffe/spire-crds \
  --namespace "${SPIRE_NAMESPACE}" \
  --create-namespace \
  --timeout "${HELM_TIMEOUT}" \
  --wait

log "Installing SPIRE stack (release=${SPIRE_RELEASE}, ns=${SPIRE_NAMESPACE})..."
run_helm upgrade --install "${SPIRE_RELEASE}" spiffe/spire \
  --namespace "${SPIRE_NAMESPACE}" \
  --timeout "${HELM_TIMEOUT}" \
  --wait

if ! $DRY_RUN; then
  log "SPIRE pods:"
  kubectl get pods -n "${SPIRE_NAMESPACE}" 2>/dev/null | head -10 || true
  log "Enable on Ragnarok backend: ./scripts/enable-confidential-production.sh --spire"
fi

log "Done. See deploy/confidential/spire/README.md"
