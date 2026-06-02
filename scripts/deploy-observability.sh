#!/usr/bin/env bash
# ============================================================================
# deploy-observability.sh — Deploy the Aether observability stack
# ============================================================================
# Usage:
#   ./scripts/deploy-observability.sh [command] [namespace]
#
# Commands:
#   deploy   Apply Prometheus, AlertManager, Grafana, Loki resources (default)
#   expose   Patch Grafana/Prometheus/Alertmanager services to NodePort (no redeploy)
#   status   Show resource status
#   urls     Print access commands
#   delete   Remove the observability namespace
#
# Environment:
#   AETHER_OBSERVABILITY_EXPOSE — clusterip | nodeport (default clusterip)
#   AETHER_GRAFANA_NODE_PORT — Grafana NodePort (default 30300)
#   AETHER_PROMETHEUS_NODE_PORT — Prometheus NodePort (default 30091)
#   AETHER_ALERTMANAGER_NODE_PORT — Alertmanager NodePort (default 30093)
#   AETHER_OBSERVABILITY_HOST — public host for printed URLs (e.g. 212.8.252.194)
#   AETHER_OPEN_FIREWALL=1 — best-effort ufw allow for observability NodePorts (remote)
#   AETHER_SKIP_OBSERVABILITY_HEALTH=1 — skip external curl probes after NodePort expose
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
OBSERVABILITY_EXPOSE="${AETHER_OBSERVABILITY_EXPOSE:-clusterip}"
GRAFANA_NODE_PORT="${AETHER_GRAFANA_NODE_PORT:-30300}"
PROMETHEUS_NODE_PORT="${AETHER_PROMETHEUS_NODE_PORT:-30091}"
ALERTMANAGER_NODE_PORT="${AETHER_ALERTMANAGER_NODE_PORT:-30093}"
OBSERVABILITY_HOST="${AETHER_OBSERVABILITY_HOST:-}"

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

observability_node_port() {
  local svc="$1"
  ${KUBECTL} -n "${NAMESPACE}" get svc "${svc}" -o jsonpath='{.spec.ports[0].nodePort}' 2>/dev/null || true
}

expose_nodeport_services() {
  [ "${OBSERVABILITY_EXPOSE}" = "nodeport" ] || return 0

  step "Exposing Grafana, Prometheus, and Alertmanager via NodePort"
  ${KUBECTL} patch svc grafana -n "${NAMESPACE}" --type=merge -p \
    "{\"spec\":{\"type\":\"NodePort\",\"ports\":[{\"name\":\"web\",\"port\":3000,\"targetPort\":\"web\",\"protocol\":\"TCP\",\"nodePort\":${GRAFANA_NODE_PORT}}]}}"
  ${KUBECTL} patch svc prometheus -n "${NAMESPACE}" --type=merge -p \
    "{\"spec\":{\"type\":\"NodePort\",\"ports\":[{\"name\":\"web\",\"port\":9090,\"targetPort\":\"web\",\"protocol\":\"TCP\",\"nodePort\":${PROMETHEUS_NODE_PORT}}]}}"
  ${KUBECTL} patch svc alertmanager -n "${NAMESPACE}" --type=merge -p \
    "{\"spec\":{\"type\":\"NodePort\",\"ports\":[{\"name\":\"web\",\"port\":9093,\"targetPort\":\"web\",\"protocol\":\"TCP\",\"nodePort\":${ALERTMANAGER_NODE_PORT}}]}}"
  info "NodePorts: Grafana=${GRAFANA_NODE_PORT} Prometheus=${PROMETHEUS_NODE_PORT} Alertmanager=${ALERTMANAGER_NODE_PORT}"

  if [ "${AETHER_OPEN_FIREWALL:-}" = "1" ] && command -v ufw >/dev/null 2>&1 && sudo -n ufw status >/dev/null 2>&1; then
    for p in "${GRAFANA_NODE_PORT}" "${PROMETHEUS_NODE_PORT}" "${ALERTMANAGER_NODE_PORT}"; do
      sudo -n ufw allow "${p}/tcp" >/dev/null 2>&1 || true
    done
    info "Requested ufw allow for observability NodePorts"
  fi
}

verify_nodeport_health() {
  [ "${OBSERVABILITY_EXPOSE}" = "nodeport" ] || return 0
  [ "${AETHER_SKIP_OBSERVABILITY_HEALTH:-}" = "1" ] || [ -z "${OBSERVABILITY_HOST}" ] || return 0
  command -v curl >/dev/null 2>&1 || return 0

  local grafana_url="http://${OBSERVABILITY_HOST}:${GRAFANA_NODE_PORT}/api/health"
  local prom_url="http://${OBSERVABILITY_HOST}:${PROMETHEUS_NODE_PORT}/-/healthy"
  local grafana_code prom_code
  grafana_code="$(curl -sS -m 12 -o /dev/null -w '%{http_code}' "${grafana_url}" 2>/dev/null || printf '%s' "000")"
  prom_code="$(curl -sS -m 12 -o /dev/null -w '%{http_code}' "${prom_url}" 2>/dev/null || printf '%s' "000")"
  if [ "${grafana_code}" = "200" ]; then
    info "Grafana health → ${grafana_url} (200)"
  else
    warn "Grafana health → ${grafana_url} returned '${grafana_code}' (open TCP ${GRAFANA_NODE_PORT} in cloud firewall if needed)"
  fi
  if [ "${prom_code}" = "200" ]; then
    info "Prometheus health → ${prom_url} (200)"
  else
    warn "Prometheus health → ${prom_url} returned '${prom_code}' (open TCP ${PROMETHEUS_NODE_PORT} in cloud firewall if needed)"
  fi
}

show_urls() {
  echo ""
  if [ "${OBSERVABILITY_EXPOSE}" = "nodeport" ] && [ -n "${OBSERVABILITY_HOST}" ]; then
    echo "  Public NodePort URLs (${OBSERVABILITY_HOST}):"
    echo "    Grafana:       http://${OBSERVABILITY_HOST}:${GRAFANA_NODE_PORT}  (admin / admin)"
    echo "    Prometheus:    http://${OBSERVABILITY_HOST}:${PROMETHEUS_NODE_PORT}"
    echo "    Alertmanager:  http://${OBSERVABILITY_HOST}:${ALERTMANAGER_NODE_PORT}"
    echo ""
  elif [ "${OBSERVABILITY_EXPOSE}" = "nodeport" ]; then
    local gnp pnp anp
    gnp="$(observability_node_port grafana)"
    pnp="$(observability_node_port prometheus)"
    anp="$(observability_node_port alertmanager)"
    echo "  NodePort services:"
    echo "    Grafana:       http://<host>:${gnp:-${GRAFANA_NODE_PORT}}"
    echo "    Prometheus:    http://<host>:${pnp:-${PROMETHEUS_NODE_PORT}}"
    echo "    Alertmanager:  http://<host>:${anp:-${ALERTMANAGER_NODE_PORT}}"
    echo ""
  fi
  echo "  Port-forward (local kubectl):"
  echo "    Grafana:       kubectl port-forward -n ${NAMESPACE} svc/grafana 3000:3000"
  echo "    Prometheus:    kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090"
  echo "    Alertmanager:  kubectl port-forward -n ${NAMESPACE} svc/alertmanager 9093:9093"
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

  expose_nodeport_services
  verify_nodeport_health

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
  expose)
    require_cmd "${KUBECTL}"
    OBSERVABILITY_EXPOSE="nodeport"
    expose_nodeport_services
    verify_nodeport_health
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
