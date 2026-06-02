#!/usr/bin/env bash
# ============================================================================
# deploy-observability.sh — Deploy the Aether observability stack
# ============================================================================
# Usage:
#   ./scripts/deploy-observability.sh [command] [namespace]
#
# Commands:
#   deploy   Apply Prometheus, AlertManager, Grafana, Loki resources (default)
#   status   Show resource status
#   urls     Print access commands
#   delete   Remove the observability namespace
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"
EXAMPLES_DIR="${REPO_ROOT}/examples/observability"

COMMAND="${1:-deploy}"
if [ "$#" -gt 0 ]; then
  shift
fi
NAMESPACE="${1:-${OBSERVABILITY_NAMESPACE:-observability}}"
KUBECTL="${KUBECTL:-kubectl}"

info() { aether_ok "$*"; }
warn() { aether_warn "$*"; }
step() { aether_step "$*"; }
error() { aether_die "$*"; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || error "Missing required command: $1"
}

wait_for_selector() {
  local selector="$1"
  local timeout="${2:-300s}"
  ${KUBECTL} wait --for=condition=ready pod -l "${selector}" -n "${NAMESPACE}" --timeout="${timeout}"
}

show_urls() {
  echo ""
  echo "  Grafana:"
  echo "    kubectl port-forward -n ${NAMESPACE} svc/grafana 3000:3000"
  echo "    http://localhost:3000"
  echo ""
  echo "  Prometheus:"
  echo "    kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090"
  echo "    http://localhost:9090"
  echo ""
  echo "  AlertManager:"
  echo "    kubectl port-forward -n ${NAMESPACE} svc/alertmanager 9093:9093"
  echo "    http://localhost:9093"
  echo ""
}

deploy_stack() {
  require_cmd "${KUBECTL}"
  [ -d "${EXAMPLES_DIR}" ] || error "Missing observability manifests at ${EXAMPLES_DIR}"

  step "Preparing namespace"
  ${KUBECTL} create namespace "${NAMESPACE}" --dry-run=client -o yaml | ${KUBECTL} apply -f -
  info "Namespace ready: ${NAMESPACE}"

  step "Applying manifests"
  if [ -f "${REPO_ROOT}/grafana/dashboard.json" ]; then
    ${KUBECTL} create configmap aether-dashboard \
      --from-file=aether-dashboard.json="${REPO_ROOT}/grafana/dashboard.json" \
      -n "${NAMESPACE}" \
      --dry-run=client -o yaml | ${KUBECTL} apply -f -
    info "Grafana dashboard ConfigMap ready"
  else
    warn "Missing grafana/dashboard.json — Grafana will start without pre-provisioned dashboards"
  fi
  ${KUBECTL} apply -n "${NAMESPACE}" -f "${EXAMPLES_DIR}/prometheus.yaml"
  ${KUBECTL} apply -n "${NAMESPACE}" -f "${EXAMPLES_DIR}/alertmanager.yaml"
  ${KUBECTL} apply -n "${NAMESPACE}" -f "${EXAMPLES_DIR}/grafana.yaml"
  ${KUBECTL} apply -n "${NAMESPACE}" -f "${EXAMPLES_DIR}/loki.yaml"
  info "Observability manifests applied"

  step "Waiting for core components"
  wait_for_selector "app=prometheus"
  wait_for_selector "app=alertmanager"
  wait_for_selector "app=grafana"
  wait_for_selector "app=loki"
  info "Core components are ready"

  if ${KUBECTL} get daemonset promtail -n "${NAMESPACE}" >/dev/null 2>&1; then
    ${KUBECTL} rollout status daemonset/promtail -n "${NAMESPACE}" --timeout=300s || warn "Promtail did not fully roll out before timeout"
  fi

  step "Status"
  ${KUBECTL} get pods,svc -n "${NAMESPACE}"
  show_urls
}

case "${COMMAND}" in
  deploy)
    echo ""
    aether_sparkle_line "Observability"
    echo -e "${A_CYN}${A_BLD}     📊  Prometheus · Grafana · Loki · Alertmanager${A_RST}"
    echo ""
    deploy_stack
    ;;
  status)
    require_cmd "${KUBECTL}"
    ${KUBECTL} get pods,svc,deploy,daemonset -n "${NAMESPACE}"
    ;;
  urls)
    show_urls
    ;;
  delete)
    require_cmd "${KUBECTL}"
    ${KUBECTL} delete namespace "${NAMESPACE}" --ignore-not-found
    info "Deleted namespace ${NAMESPACE}"
    ;;
  *)
    error "Unknown command: ${COMMAND}"
    ;;
esac
