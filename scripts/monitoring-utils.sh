#!/usr/bin/env bash
# ============================================================================
# monitoring-utils.sh — Utility commands for the observability stack
# ============================================================================

set -euo pipefail

NAMESPACE="${NAMESPACE:-observability}"
KUBECTL="${KUBECTL:-kubectl}"

info() { echo "  [✓] $*"; }
warn() { echo "  [!] $*"; }
error() { echo "  [✗] $*" >&2; exit 1; }
section() { echo ""; echo "== $*"; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || error "Missing required command: $1"
}

require_stack() {
  ${KUBECTL} get namespace "${NAMESPACE}" >/dev/null 2>&1 || error "Namespace not found: ${NAMESPACE}"
}

with_port_forward() {
  local service="$1"
  local local_port="$2"
  local remote_port="$3"
  local callback="$4"
  ${KUBECTL} port-forward -n "${NAMESPACE}" "svc/${service}" "${local_port}:${remote_port}" >/dev/null 2>&1 &
  local pf_pid=$!
  trap 'kill "${pf_pid}" >/dev/null 2>&1 || true' RETURN
  sleep 2
  "${callback}"
}

component_pod() {
  local selector="$1"
  ${KUBECTL} get pods -n "${NAMESPACE}" -l "${selector}" -o jsonpath='{.items[0].metadata.name}'
}

check_health() {
  section "Observability Stack Health"
  local components=("prometheus" "grafana" "alertmanager" "loki")
  local component
  for component in "${components[@]}"; do
    if ${KUBECTL} get deployment "${component}" -n "${NAMESPACE}" >/dev/null 2>&1; then
      local ready desired
      ready="$(${KUBECTL} get deployment "${component}" -n "${NAMESPACE}" -o jsonpath='{.status.readyReplicas}')"
      desired="$(${KUBECTL} get deployment "${component}" -n "${NAMESPACE}" -o jsonpath='{.status.replicas}')"
      ready="${ready:-0}"
      desired="${desired:-0}"
      if [ "${ready}" -ge 1 ]; then
        info "${component}: ${ready}/${desired} ready"
      else
        warn "${component}: ${ready}/${desired} ready"
      fi
    else
      warn "${component}: not deployed"
    fi
  done

  if ${KUBECTL} get daemonset promtail -n "${NAMESPACE}" >/dev/null 2>&1; then
    local desired ready
    desired="$(${KUBECTL} get daemonset promtail -n "${NAMESPACE}" -o jsonpath='{.status.desiredNumberScheduled}')"
    ready="$(${KUBECTL} get daemonset promtail -n "${NAMESPACE}" -o jsonpath='{.status.numberReady}')"
    info "promtail: ${ready:-0}/${desired:-0} ready"
  fi
}

show_targets() {
  require_cmd curl
  require_cmd jq
  with_port_forward prometheus 9090 9090 _show_targets
}

_show_targets() {
  section "Prometheus Targets"
  curl -s http://127.0.0.1:9090/api/v1/targets | jq -r '
    .data.activeTargets[] |
    "\(.labels.job)\t\(.health)\t\(.labels.instance // .labels.pod // "n/a")"
  '
}

show_alerts() {
  require_cmd curl
  require_cmd jq
  with_port_forward prometheus 9090 9090 _show_alerts
}

_show_alerts() {
  section "Active Alerts"
  local alerts
  alerts="$(curl -s http://127.0.0.1:9090/api/v1/alerts | jq -r '
    .data.alerts[]? |
    select(.state == "firing") |
    "\(.labels.alertname)\t\(.labels.severity // "n/a")\t\(.annotations.summary // "")"
  ')"
  if [ -z "${alerts}" ]; then
    info "No active alerts"
  else
    echo "${alerts}"
  fi
}

view_logs() {
  local component="${1:-}"
  [ -n "${component}" ] || error "Usage: $0 logs <component>"
  local selector="app=${component}"
  [ "${component}" = "promtail" ] && selector="app=promtail"
  local pod
  pod="$(component_pod "${selector}")"
  [ -n "${pod}" ] || error "No pod found for ${component}"
  ${KUBECTL} logs -n "${NAMESPACE}" "${pod}" --tail=100 -f
}

restart_component() {
  local component="${1:-}"
  [ -n "${component}" ] || error "Usage: $0 restart <component>"
  if [ "${component}" = "promtail" ]; then
    ${KUBECTL} rollout restart daemonset/promtail -n "${NAMESPACE}"
  else
    ${KUBECTL} rollout restart deployment/"${component}" -n "${NAMESPACE}"
  fi
  info "Restart initiated for ${component}"
}

show_storage() {
  section "PVC Usage"
  ${KUBECTL} get pvc -n "${NAMESPACE}" || true
}

backup_prometheus() {
  local backup_dir="${1:-/tmp/prometheus-backup-$(date +%Y%m%d-%H%M%S)}"
  local pod
  pod="$(component_pod 'app=prometheus')"
  [ -n "${pod}" ] || error "No Prometheus pod found"
  section "Prometheus Backup"
  ${KUBECTL} exec -n "${NAMESPACE}" "${pod}" -- curl -fsS -XPOST http://127.0.0.1:9090/api/v1/admin/tsdb/snapshot >/dev/null
  local snapshot
  snapshot="$(${KUBECTL} exec -n "${NAMESPACE}" "${pod}" -- sh -lc 'ls -t /prometheus/snapshots | head -1')"
  [ -n "${snapshot}" ] || error "No snapshot created"
  ${KUBECTL} cp -n "${NAMESPACE}" "${pod}:/prometheus/snapshots/${snapshot}" "${backup_dir}"
  info "Backup copied to ${backup_dir}"
}

query_metrics() {
  local query="${1:-}"
  [ -n "${query}" ] || error "Usage: $0 query '<promql>'"
  require_cmd curl
  require_cmd jq
  with_port_forward prometheus 9090 9090 _query_metrics
}

_query_metrics() {
  local query="${1:-${PROMQL_QUERY:-}}"
  curl -s -G --data-urlencode "query=${query}" http://127.0.0.1:9090/api/v1/query | jq -r '
    .data.result[]? |
    "\(.metric | to_entries | map("\(.key)=\(.value)") | join(","))\t\(.value[1])"
  '
}

test_alertmanager_config() {
  local pod
  pod="$(component_pod 'app=alertmanager')"
  [ -n "${pod}" ] || error "No AlertManager pod found"
  ${KUBECTL} exec -n "${NAMESPACE}" "${pod}" -- amtool check-config /etc/alertmanager/alertmanager.yml
}

send_test_alert() {
  require_cmd curl
  with_port_forward alertmanager 9093 9093 _send_test_alert
}

_send_test_alert() {
  curl -fsS -XPOST http://127.0.0.1:9093/api/v2/alerts \
    -H 'Content-Type: application/json' \
    -d '[{"labels":{"alertname":"AetherTestAlert","severity":"warning","component":"aether"},"annotations":{"summary":"Aether test alert","description":"Monitoring connectivity test"}}]'
  info "Test alert submitted"
}

show_urls() {
  echo "Grafana:      kubectl port-forward -n ${NAMESPACE} svc/grafana 3000:3000"
  echo "Prometheus:   kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090"
  echo "AlertManager: kubectl port-forward -n ${NAMESPACE} svc/alertmanager 9093:9093"
}

usage() {
  cat <<EOF
Usage: $0 <command> [options]

Commands:
  health
  targets
  alerts
  logs <component>
  restart <component>
  storage
  backup [dir]
  query '<promql>'
  test-config
  test-alert
  urls
EOF
}

require_cmd "${KUBECTL}"
require_stack

case "${1:-}" in
  health|status) check_health ;;
  targets) show_targets ;;
  alerts) show_alerts ;;
  logs) view_logs "${2:-}" ;;
  restart) restart_component "${2:-}" ;;
  storage) show_storage ;;
  backup) backup_prometheus "${2:-}" ;;
  query)
    export PROMQL_QUERY="${2:-}"
    query_metrics "${2:-}"
    ;;
  test-config) test_alertmanager_config ;;
  test-alert) send_test_alert ;;
  urls) show_urls ;;
  ""|--help|-h)
    usage
    ;;
  *)
    error "Unknown command: ${1}"
    ;;
esac
