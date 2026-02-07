#!/bin/bash
# Monitoring Utilities for Orchestr8
# Common operations for observability stack management

set -e

NAMESPACE=${NAMESPACE:-observability}

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_header() {
    echo ""
    echo "========================================="
    echo "$1"
    echo "========================================="
}

# Function to check component health
check_health() {
    print_header "Observability Stack Health"

    components=("prometheus" "grafana" "alertmanager" "loki")

    for component in "${components[@]}"; do
        if kubectl get deployment ${component} -n ${NAMESPACE} &>/dev/null; then
            status=$(kubectl get deployment ${component} -n ${NAMESPACE} -o jsonpath='{.status.conditions[?(@.type=="Available")].status}')
            if [ "$status" == "True" ]; then
                print_status "${GREEN}" "✅ ${component}: Ready"
            else
                print_status "${RED}" "❌ ${component}: Not Ready"
            fi
        else
            print_status "${YELLOW}" "⚠️  ${component}: Not Deployed"
        fi
    done

    # Check Promtail DaemonSet
    if kubectl get daemonset promtail -n ${NAMESPACE} &>/dev/null; then
        desired=$(kubectl get daemonset promtail -n ${NAMESPACE} -o jsonpath='{.status.desiredNumberScheduled}')
        ready=$(kubectl get daemonset promtail -n ${NAMESPACE} -o jsonpath='{.status.numberReady}')
        if [ "$desired" == "$ready" ]; then
            print_status "${GREEN}" "✅ promtail: ${ready}/${desired} pods ready"
        else
            print_status "${YELLOW}" "⚠️  promtail: ${ready}/${desired} pods ready"
        fi
    fi
}

# Function to check Prometheus targets
check_targets() {
    print_header "Prometheus Targets Status"

    # Port forward Prometheus
    kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090 &>/dev/null &
    PF_PID=$!
    sleep 3

    # Query targets API
    curl -s http://localhost:9090/api/v1/targets | jq -r '
        .data.activeTargets[] |
        "\(.labels.job)\t\(.health)\t\(.labels.instance // .labels.pod)"
    ' | column -t -s $'\t' || print_status "${RED}" "Failed to get targets"

    # Cleanup
    kill $PF_PID 2>/dev/null
}

# Function to check alert status
check_alerts() {
    print_header "Active Alerts"

    # Port forward Prometheus
    kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090 &>/dev/null &
    PF_PID=$!
    sleep 3

    # Query alerts API
    alerts=$(curl -s http://localhost:9090/api/v1/alerts | jq -r '
        .data.alerts[] |
        select(.state == "firing") |
        "\(.labels.alertname)\t\(.labels.severity)\t\(.annotations.summary)"
    ')

    if [ -z "$alerts" ]; then
        print_status "${GREEN}" "✅ No active alerts"
    else
        echo "$alerts" | column -t -s $'\t'
    fi

    # Cleanup
    kill $PF_PID 2>/dev/null
}

# Function to view logs
view_logs() {
    local component=$1

    if [ -z "$component" ]; then
        print_status "${RED}" "Usage: $0 logs <component>"
        print_status "${YELLOW}" "Components: prometheus, grafana, alertmanager, loki, promtail"
        return 1
    fi

    print_header "Logs: ${component}"

    if [ "$component" == "promtail" ]; then
        # DaemonSet - show logs from first pod
        pod=$(kubectl get pods -n ${NAMESPACE} -l app=promtail -o jsonpath='{.items[0].metadata.name}')
    else
        pod=$(kubectl get pods -n ${NAMESPACE} -l app=${component} -o jsonpath='{.items[0].metadata.name}')
    fi

    if [ -z "$pod" ]; then
        print_status "${RED}" "No pod found for ${component}"
        return 1
    fi

    kubectl logs -n ${NAMESPACE} ${pod} --tail=100 -f
}

# Function to restart component
restart_component() {
    local component=$1

    if [ -z "$component" ]; then
        print_status "${RED}" "Usage: $0 restart <component>"
        return 1
    fi

    print_status "${YELLOW}" "Restarting ${component}..."

    if [ "$component" == "promtail" ]; then
        kubectl rollout restart daemonset/${component} -n ${NAMESPACE}
    else
        kubectl rollout restart deployment/${component} -n ${NAMESPACE}
    fi

    print_status "${GREEN}" "✅ ${component} restart initiated"
}

# Function to check storage usage
check_storage() {
    print_header "Storage Usage"

    pvcs=$(kubectl get pvc -n ${NAMESPACE} -o json)

    echo "$pvcs" | jq -r '.items[] |
        "\(.metadata.name)\t\(.spec.resources.requests.storage)\t\(.status.capacity.storage // "N/A")"
    ' | column -t -s $'\t' | while read line; do
        echo "  $line"
    done
}

# Function to backup Prometheus data
backup_prometheus() {
    local backup_dir=${1:-/tmp/prometheus-backup-$(date +%Y%m%d-%H%M%S)}

    print_header "Backing up Prometheus Data"

    # Create snapshot
    pod=$(kubectl get pods -n ${NAMESPACE} -l app=prometheus -o jsonpath='{.items[0].metadata.name}')

    print_status "${YELLOW}" "Creating snapshot..."
    kubectl exec -n ${NAMESPACE} ${pod} -- \
        curl -XPOST http://localhost:9090/api/v1/admin/tsdb/snapshot

    snapshot=$(kubectl exec -n ${NAMESPACE} ${pod} -- \
        ls -t /prometheus/snapshots | head -1)

    print_status "${YELLOW}" "Copying snapshot ${snapshot}..."
    kubectl cp -n ${NAMESPACE} ${pod}:/prometheus/snapshots/${snapshot} ${backup_dir}

    print_status "${GREEN}" "✅ Backup completed: ${backup_dir}"
}

# Function to query metrics
query_metrics() {
    local query=$1

    if [ -z "$query" ]; then
        print_status "${RED}" "Usage: $0 query '<promql>'"
        return 1
    fi

    # Port forward Prometheus
    kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090 &>/dev/null &
    PF_PID=$!
    sleep 3

    result=$(curl -s -G --data-urlencode "query=${query}" http://localhost:9090/api/v1/query)

    echo "$result" | jq -r '.data.result[] |
        "\(.metric | to_entries | map("\(.key)=\(.value)") | join(","))\t\(.value[1])"
    ' | column -t -s $'\t'

    # Cleanup
    kill $PF_PID 2>/dev/null
}

# Function to test alertmanager config
test_alertmanager_config() {
    print_header "Testing AlertManager Configuration"

    # Get config
    config=$(kubectl get configmap alertmanager-config -n ${NAMESPACE} -o jsonpath='{.data.alertmanager\.yml}')

    # Test with amtool
    pod=$(kubectl get pods -n ${NAMESPACE} -l app=alertmanager -o jsonpath='{.items[0].metadata.name}')

    print_status "${YELLOW}" "Checking configuration..."
    kubectl exec -n ${NAMESPACE} ${pod} -- amtool check-config /etc/alertmanager/alertmanager.yml

    if [ $? -eq 0 ]; then
        print_status "${GREEN}" "✅ AlertManager configuration is valid"
    else
        print_status "${RED}" "❌ AlertManager configuration has errors"
        return 1
    fi
}

# Function to send test alert
send_test_alert() {
    print_header "Sending Test Alert"

    # Port forward Prometheus
    kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090 &>/dev/null &
    PF_PID=$!
    sleep 3

    # Send test alert
    curl -XPOST http://localhost:9090/api/v1/alerts -d '[{
        "labels": {
            "alertname": "TestAlert",
            "severity": "warning",
            "component": "orchestr8"
        },
        "annotations": {
            "summary": "This is a test alert",
            "description": "Testing alert routing and notifications"
        }
    }]'

    print_status "${GREEN}" "✅ Test alert sent"
    print_status "${YELLOW}" "Check AlertManager UI at http://localhost:9093"

    # Cleanup
    kill $PF_PID 2>/dev/null
}

# Function to show dashboard URLs
show_urls() {
    print_header "Service URLs"

    echo ""
    echo "Port-forward commands:"
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

# Main command handler
case ${1} in
    health|status)
        check_health
        ;;
    targets)
        check_targets
        ;;
    alerts)
        check_alerts
        ;;
    logs)
        view_logs "$2"
        ;;
    restart)
        restart_component "$2"
        ;;
    storage)
        check_storage
        ;;
    backup)
        backup_prometheus "$2"
        ;;
    query)
        query_metrics "$2"
        ;;
    test-config)
        test_alertmanager_config
        ;;
    test-alert)
        send_test_alert
        ;;
    urls)
        show_urls
        ;;
    *)
        echo "Orchestr8 Monitoring Utilities"
        echo ""
        echo "Usage: $0 <command> [options]"
        echo ""
        echo "Commands:"
        echo "  health              - Check observability stack health"
        echo "  targets             - Show Prometheus scrape targets"
        echo "  alerts              - Show active alerts"
        echo "  logs <component>    - View component logs"
        echo "  restart <component> - Restart a component"
        echo "  storage             - Show storage usage"
        echo "  backup [dir]        - Backup Prometheus data"
        echo "  query '<promql>'    - Run PromQL query"
        echo "  test-config         - Test AlertManager configuration"
        echo "  test-alert          - Send test alert"
        echo "  urls                - Show service URLs"
        echo ""
        echo "Components: prometheus, grafana, alertmanager, loki, promtail"
        echo ""
        echo "Examples:"
        echo "  $0 health"
        echo "  $0 logs prometheus"
        echo "  $0 query 'up'"
        echo "  $0 backup /backups/prometheus"
        ;;
esac
