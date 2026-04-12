#!/bin/bash
# Deploy all services in order with health checks
# Usage: ./scripts/deploy-all.sh [environment]

set -e

ENVIRONMENT=${1:-staging}
WORKLOADS_DIR="workloads"
NAMESPACE="aether-${ENVIRONMENT}"

echo "🚀 Deploying all services to ${ENVIRONMENT}"
echo "Namespace: ${NAMESPACE}"
echo "---"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print colored output
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

# Function to wait for deployment
wait_for_deployment() {
    local service_name=$1
    local timeout=300

    print_status "info" "Waiting for ${service_name} to be ready..."

    if kubectl wait --for=condition=ready pod \
        -l app=${service_name} \
        -n ${NAMESPACE} \
        --timeout=${timeout}s 2>/dev/null; then
        print_status "success" "${service_name} is ready"
        return 0
    else
        print_status "error" "${service_name} failed to become ready"
        return 1
    fi
}

# Function to check health endpoint
check_health() {
    local service_name=$1
    local max_retries=10
    local retry=0

    while [ $retry -lt $max_retries ]; do
        if kubectl exec -n ${NAMESPACE} \
            $(kubectl get pod -n ${NAMESPACE} -l app=${service_name} -o jsonpath='{.items[0].metadata.name}') \
            -- curl -sf http://localhost:8080/health > /dev/null 2>&1; then
            print_status "success" "${service_name} health check passed"
            return 0
        fi
        retry=$((retry + 1))
        sleep 5
    done

    print_status "error" "${service_name} health check failed"
    return 1
}

# Pre-deployment checks
print_status "info" "Running pre-deployment checks..."

# Check if kubectl is installed
if ! command -v kubectl &> /dev/null; then
    print_status "error" "kubectl not found. Please install kubectl."
    exit 1
fi

# Check if aether is installed
if ! command -v aether &> /dev/null; then
    print_status "error" "aether not found. Please install aether."
    exit 1
fi

# Check if namespace exists
if ! kubectl get namespace ${NAMESPACE} &> /dev/null; then
    print_status "info" "Creating namespace ${NAMESPACE}..."
    kubectl create namespace ${NAMESPACE}
fi

# Create backup before deployment
print_status "info" "Creating pre-deployment backup..."
BACKUP_NAME="pre-deploy-all-$(date +%Y%m%d-%H%M%S)"
aether backup -n "${BACKUP_NAME}" -d "Pre-deployment backup for ${ENVIRONMENT}"
print_status "success" "Backup created: ${BACKUP_NAME}"

# Define deployment order (infrastructure first, then services)
INFRASTRUCTURE=(
    "infrastructure/postgres.yaml"
    "infrastructure/redis.yaml"
    "infrastructure/rabbitmq.yaml"
)

SERVICES=(
    "services/api-gateway.yaml"
    "services/user-service.yaml"
    "services/product-service.yaml"
    "services/order-service.yaml"
    "services/payment-service.yaml"
)

# Deploy infrastructure
print_status "info" "Deploying infrastructure components..."
for workload in "${INFRASTRUCTURE[@]}"; do
    if [ ! -f "${WORKLOADS_DIR}/${workload}" ]; then
        print_status "error" "Workload file not found: ${workload}"
        continue
    fi

    service_name=$(basename "${workload}" .yaml)
    print_status "info" "Deploying ${service_name}..."

    if aether -s "${WORKLOADS_DIR}/${workload}" run; then
        print_status "success" "${service_name} deployed"

        # Wait for infrastructure to be ready
        if wait_for_deployment "${service_name}"; then
            sleep 10  # Additional stabilization time
        else
            print_status "error" "Failed to deploy ${service_name}"
            exit 1
        fi
    else
        print_status "error" "Failed to deploy ${service_name}"
        exit 1
    fi
done

print_status "success" "Infrastructure deployed successfully"
echo "---"

# Deploy services
print_status "info" "Deploying application services..."
FAILED_SERVICES=()

for workload in "${SERVICES[@]}"; do
    if [ ! -f "${WORKLOADS_DIR}/${workload}" ]; then
        print_status "error" "Workload file not found: ${workload}"
        FAILED_SERVICES+=("${workload}")
        continue
    fi

    service_name=$(basename "${workload}" .yaml)
    print_status "info" "Deploying ${service_name}..."

    if aether -s "${WORKLOADS_DIR}/${workload}" run; then
        print_status "success" "${service_name} deployed"

        # Wait for service to be ready
        if wait_for_deployment "${service_name}"; then
            # Run health check
            if check_health "${service_name}"; then
                print_status "success" "${service_name} is healthy"
            else
                print_status "error" "${service_name} health check failed"
                FAILED_SERVICES+=("${service_name}")
            fi
        else
            print_status "error" "${service_name} failed to become ready"
            FAILED_SERVICES+=("${service_name}")
        fi
    else
        print_status "error" "Failed to deploy ${service_name}"
        FAILED_SERVICES+=("${service_name}")
    fi

    sleep 5  # Brief pause between deployments
done

echo "---"
print_status "info" "Deployment Summary"
echo "---"

# Show final status
kubectl get pods -n ${NAMESPACE}

# Check for failures
if [ ${#FAILED_SERVICES[@]} -eq 0 ]; then
    print_status "success" "All services deployed successfully!"
    print_status "info" "Backup available: ${BACKUP_NAME}"
    exit 0
else
    print_status "error" "Some services failed to deploy:"
    for service in "${FAILED_SERVICES[@]}"; do
        echo "  - ${service}"
    done
    print_status "info" "Backup available for rollback: ${BACKUP_NAME}"
    print_status "info" "To rollback: aether restore ~/.aether/backups/${BACKUP_NAME}.json"
    exit 1
fi
