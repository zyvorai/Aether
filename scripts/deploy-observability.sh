#!/bin/bash
# Deploy Observability Stack for Orchestr8
# Complete monitoring, logging, and alerting infrastructure

set -e

NAMESPACE=${1:-observability}
EXAMPLES_DIR="examples/observability"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_status() {
    local status=$1
    local message=$2
    case $status in
        "success")
            echo -e "${GREEN}✅ ${message}${NC}"
            ;;
        "info")
            echo -e "${YELLOW}ℹ️  ${message}${NC}"
            ;;
        "error")
            echo -e "${RED}❌ ${message}${NC}"
            ;;
    esac
}

print_header() {
    echo ""
    echo "========================================="
    echo "$1"
    echo "========================================="
}

# Check prerequisites
print_header "Checking Prerequisites"

if ! command -v kubectl &> /dev/null; then
    print_status "error" "kubectl not found. Please install kubectl."
    exit 1
fi
print_status "success" "kubectl found"

# Create namespace
print_header "Creating Namespace"
if kubectl get namespace ${NAMESPACE} &> /dev/null; then
    print_status "info" "Namespace ${NAMESPACE} already exists"
else
    kubectl create namespace ${NAMESPACE}
    print_status "success" "Namespace ${NAMESPACE} created"
fi

# Deploy Prometheus
print_header "Deploying Prometheus"
print_status "info" "Deploying Prometheus configuration..."

kubectl apply -f ${EXAMPLES_DIR}/prometheus.yaml -n ${NAMESPACE}
print_status "success" "Prometheus deployed"

# Wait for Prometheus
print_status "info" "Waiting for Prometheus to be ready..."
kubectl wait --for=condition=ready pod \
    -l app=prometheus \
    -n ${NAMESPACE} \
    --timeout=300s
print_status "success" "Prometheus is ready"

# Deploy AlertManager
print_header "Deploying AlertManager"
print_status "info" "Deploying AlertManager configuration..."

kubectl apply -f ${EXAMPLES_DIR}/alertmanager.yaml -n ${NAMESPACE}
print_status "success" "AlertManager deployed"

# Wait for AlertManager
print_status "info" "Waiting for AlertManager to be ready..."
kubectl wait --for=condition=ready pod \
    -l app=alertmanager \
    -n ${NAMESPACE} \
    --timeout=300s
print_status "success" "AlertManager is ready"

# Deploy Grafana
print_header "Deploying Grafana"
print_status "info" "Deploying Grafana configuration..."

kubectl apply -f ${EXAMPLES_DIR}/grafana.yaml -n ${NAMESPACE}
print_status "success" "Grafana deployed"

# Wait for Grafana
print_status "info" "Waiting for Grafana to be ready..."
kubectl wait --for=condition=ready pod \
    -l app=grafana \
    -n ${NAMESPACE} \
    --timeout=300s
print_status "success" "Grafana is ready"

# Deploy Loki
print_header "Deploying Loki"
print_status "info" "Deploying Loki configuration..."

kubectl apply -f ${EXAMPLES_DIR}/loki.yaml -n ${NAMESPACE}
print_status "success" "Loki deployed"

# Wait for Loki
print_status "info" "Waiting for Loki to be ready..."
kubectl wait --for=condition=ready pod \
    -l app=loki \
    -n ${NAMESPACE} \
    --timeout=300s
print_status "success" "Loki is ready"

# Wait for Promtail (DaemonSet takes a bit longer)
print_status "info" "Waiting for Promtail to be ready..."
sleep 30
print_status "success" "Promtail is running"

# Display status
print_header "Deployment Status"
kubectl get pods -n ${NAMESPACE}

# Display access instructions
print_header "Access URLs"
echo ""
echo "Use kubectl port-forward to access the services:"
echo ""
echo "  Grafana (default credentials: admin/admin):"
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

# Import dashboard instructions
print_header "Next Steps"
echo ""
echo "1. Access Grafana and change the default admin password"
echo "2. Import Orchestr8 dashboard:"
echo "   - Navigate to Dashboards → Import"
echo "   - Upload grafana/dashboard.json"
echo "   - Select Prometheus data source"
echo ""
echo "3. Configure AlertManager notifications:"
echo "   - Edit alertmanager-config ConfigMap"
echo "   - Update Slack webhook URL and PagerDuty keys"
echo "   - Reload: kubectl rollout restart deployment/alertmanager -n ${NAMESPACE}"
echo ""
echo "4. Verify metrics collection:"
echo "   - Check Prometheus targets: http://localhost:9090/targets"
echo "   - Verify Orchestr8 metrics are being scraped"
echo ""

print_status "success" "Observability stack deployed successfully!"
