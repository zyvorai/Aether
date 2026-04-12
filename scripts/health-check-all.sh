#!/bin/bash
# Comprehensive health check for all services
# Usage: ./scripts/health-check-all.sh [namespace]

set -e

NAMESPACE=${1:-aether-production}

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_header() {
    echo ""
    echo "========================================="
    echo "$1"
    echo "========================================="
}

print_status() {
    local status=$1
    local message=$2
    case $status in
        "pass")
            echo -e "${GREEN}✅ PASS${NC}: ${message}"
            ;;
        "warn")
            echo -e "${YELLOW}⚠️  WARN${NC}: ${message}"
            ;;
        "fail")
            echo -e "${RED}❌ FAIL${NC}: ${message}"
            ;;
    esac
}

# Initialize counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

run_check() {
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    if "$@"; then
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
        return 0
    else
        FAILED_CHECKS=$((FAILED_CHECKS + 1))
        return 1
    fi
}

# Check 1: Cluster connectivity
print_header "1. Cluster Connectivity"

check_cluster() {
    if kubectl cluster-info &> /dev/null; then
        print_status "pass" "Kubernetes cluster is accessible"
        return 0
    else
        print_status "fail" "Cannot connect to Kubernetes cluster"
        return 1
    fi
}
run_check check_cluster

# Check 2: Namespace exists
print_header "2. Namespace Check"

check_namespace() {
    if kubectl get namespace ${NAMESPACE} &> /dev/null; then
        print_status "pass" "Namespace ${NAMESPACE} exists"
        return 0
    else
        print_status "fail" "Namespace ${NAMESPACE} not found"
        return 1
    fi
}
run_check check_namespace

# Check 3: Pod Status
print_header "3. Pod Status"

check_pods() {
    local all_pods=$(kubectl get pods -n ${NAMESPACE} --no-headers | wc -l)
    local running_pods=$(kubectl get pods -n ${NAMESPACE} --field-selector=status.phase=Running --no-headers | wc -l)
    local pending_pods=$(kubectl get pods -n ${NAMESPACE} --field-selector=status.phase=Pending --no-headers | wc -l)
    local failed_pods=$(kubectl get pods -n ${NAMESPACE} --field-selector=status.phase=Failed --no-headers | wc -l)

    print_status "pass" "Total pods: ${all_pods}"
    print_status "pass" "Running pods: ${running_pods}"

    if [ ${pending_pods} -gt 0 ]; then
        print_status "warn" "Pending pods: ${pending_pods}"
        WARNINGS=$((WARNINGS + 1))
    fi

    if [ ${failed_pods} -gt 0 ]; then
        print_status "fail" "Failed pods: ${failed_pods}"
        kubectl get pods -n ${NAMESPACE} --field-selector=status.phase=Failed
        return 1
    fi

    # Check for crashlooping pods
    local crashloop=$(kubectl get pods -n ${NAMESPACE} -o json | \
        jq -r '.items[] | select(.status.containerStatuses[]?.state.waiting?.reason == "CrashLoopBackOff") | .metadata.name' | \
        wc -l)

    if [ ${crashloop} -gt 0 ]; then
        print_status "fail" "CrashLoopBackOff pods: ${crashloop}"
        kubectl get pods -n ${NAMESPACE} | grep CrashLoopBackOff
        return 1
    fi

    return 0
}
run_check check_pods

# Check 4: Service Endpoints
print_header "4. Service Endpoints"

check_services() {
    local services=$(kubectl get svc -n ${NAMESPACE} --no-headers | awk '{print $1}')
    local has_failures=0

    for svc in ${services}; do
        local endpoints=$(kubectl get endpoints ${svc} -n ${NAMESPACE} -o json | jq -r '.subsets[]?.addresses | length' 2>/dev/null)

        if [ -z "${endpoints}" ] || [ "${endpoints}" == "null" ]; then
            print_status "fail" "Service ${svc} has no endpoints"
            has_failures=1
        else
            print_status "pass" "Service ${svc} has ${endpoints} endpoint(s)"
        fi
    done

    return ${has_failures}
}
run_check check_services

# Check 5: Resource Usage
print_header "5. Resource Usage"

check_resources() {
    local has_issues=0

    # Get node resource usage
    kubectl top nodes 2>/dev/null | tail -n +2 | while read -r line; do
        local node=$(echo ${line} | awk '{print $1}')
        local cpu=$(echo ${line} | awk '{print $3}' | tr -d '%')
        local memory=$(echo ${line} | awk '{print $5}' | tr -d '%')

        if [ ${cpu} -gt 80 ]; then
            print_status "warn" "Node ${node} CPU usage: ${cpu}%"
            has_issues=1
        fi

        if [ ${memory} -gt 85 ]; then
            print_status "warn" "Node ${node} memory usage: ${memory}%"
            has_issues=1
        fi
    done

    # Get pod resource usage
    kubectl top pods -n ${NAMESPACE} 2>/dev/null | tail -n +2 | while read -r line; do
        local pod=$(echo ${line} | awk '{print $1}')
        local cpu=$(echo ${line} | awk '{print $2}' | sed 's/m//')
        local memory=$(echo ${line} | awk '{print $3}' | sed 's/Mi//')

        # Get resource limits
        local cpu_limit=$(kubectl get pod ${pod} -n ${NAMESPACE} -o json | \
            jq -r '.spec.containers[0].resources.limits.cpu // "unlimited"' | sed 's/m//')
        local mem_limit=$(kubectl get pod ${pod} -n ${NAMESPACE} -o json | \
            jq -r '.spec.containers[0].resources.limits.memory // "unlimited"' | sed 's/Mi//')

        # Check if approaching limits (>80%)
        if [ "${cpu_limit}" != "unlimited" ] && [ ${cpu} -gt $((cpu_limit * 80 / 100)) ]; then
            print_status "warn" "Pod ${pod} approaching CPU limit: ${cpu}m / ${cpu_limit}m"
        fi
    done

    return ${has_issues}
}
run_check check_resources || true  # Don't fail on resource warnings

# Check 6: PVC Status
print_header "6. Persistent Volume Claims"

check_pvcs() {
    local pvcs=$(kubectl get pvc -n ${NAMESPACE} --no-headers 2>/dev/null || true)

    if [ -z "${pvcs}" ]; then
        print_status "pass" "No PVCs in namespace"
        return 0
    fi

    local has_failures=0

    while read -r pvc; do
        local name=$(echo ${pvc} | awk '{print $1}')
        local status=$(echo ${pvc} | awk '{print $2}')

        if [ "${status}" != "Bound" ]; then
            print_status "fail" "PVC ${name} is ${status}"
            has_failures=1
        else
            print_status "pass" "PVC ${name} is Bound"
        fi
    done <<< "${pvcs}"

    return ${has_failures}
}
run_check check_pvcs

# Check 7: Recent Events
print_header "7. Recent Events (Warnings/Errors)"

check_events() {
    local warning_events=$(kubectl get events -n ${NAMESPACE} --sort-by='.lastTimestamp' | \
        grep -E "Warning|Error" | tail -10 || true)

    if [ -z "${warning_events}" ]; then
        print_status "pass" "No recent warning/error events"
        return 0
    else
        print_status "warn" "Recent warning/error events found:"
        echo "${warning_events}"
        WARNINGS=$((WARNINGS + 1))
        return 0
    fi
}
run_check check_events

# Check 8: Ingress Status
print_header "8. Ingress Status"

check_ingress() {
    local ingresses=$(kubectl get ingress -n ${NAMESPACE} --no-headers 2>/dev/null || true)

    if [ -z "${ingresses}" ]; then
        print_status "pass" "No ingress resources"
        return 0
    fi

    local has_failures=0

    while read -r ing; do
        local name=$(echo ${ing} | awk '{print $1}')
        local address=$(echo ${ing} | awk '{print $4}')

        if [ -z "${address}" ] || [ "${address}" == "<none>" ]; then
            print_status "fail" "Ingress ${name} has no address"
            has_failures=1
        else
            print_status "pass" "Ingress ${name}: ${address}"
        fi
    done <<< "${ingresses}"

    return ${has_failures}
}
run_check check_ingress

# Check 9: Certificate Status
print_header "9. Certificate Status"

check_certificates() {
    if ! kubectl get certificates -n ${NAMESPACE} &> /dev/null; then
        print_status "pass" "No certificates to check (cert-manager not installed)"
        return 0
    fi

    local certs=$(kubectl get certificates -n ${NAMESPACE} --no-headers 2>/dev/null || true)

    if [ -z "${certs}" ]; then
        print_status "pass" "No certificates in namespace"
        return 0
    fi

    local has_failures=0

    while read -r cert; do
        local name=$(echo ${cert} | awk '{print $1}')
        local ready=$(echo ${cert} | awk '{print $2}')

        if [ "${ready}" != "True" ]; then
            print_status "fail" "Certificate ${name} is not ready"
            has_failures=1
        else
            print_status "pass" "Certificate ${name} is ready"
        fi
    done <<< "${certs}"

    return ${has_failures}
}
run_check check_certificates

# Check 10: Database Connectivity
print_header "10. Database Connectivity"

check_database() {
    local db_pod=$(kubectl get pod -n ${NAMESPACE} -l app=postgres -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)

    if [ -z "${db_pod}" ]; then
        print_status "pass" "No database pod to check"
        return 0
    fi

    if kubectl exec -n ${NAMESPACE} ${db_pod} -- pg_isready -U postgres &> /dev/null; then
        print_status "pass" "Database is accepting connections"

        # Check connection count
        local conn_count=$(kubectl exec -n ${NAMESPACE} ${db_pod} -- psql -U postgres -t -c \
            "SELECT count(*) FROM pg_stat_activity;" 2>/dev/null | xargs)
        local max_conn=$(kubectl exec -n ${NAMESPACE} ${db_pod} -- psql -U postgres -t -c \
            "SHOW max_connections;" 2>/dev/null | xargs)

        print_status "pass" "Database connections: ${conn_count} / ${max_conn}"

        # Warn if >80% of connections used
        if [ $((conn_count * 100 / max_conn)) -gt 80 ]; then
            print_status "warn" "Database connection usage >80%"
            WARNINGS=$((WARNINGS + 1))
        fi

        return 0
    else
        print_status "fail" "Database is not accepting connections"
        return 1
    fi
}
run_check check_database

# Summary
print_header "Health Check Summary"

echo "Total Checks: ${TOTAL_CHECKS}"
echo -e "Passed: ${GREEN}${PASSED_CHECKS}${NC}"
echo -e "Failed: ${RED}${FAILED_CHECKS}${NC}"
echo -e "Warnings: ${YELLOW}${WARNINGS}${NC}"
echo ""

# Overall status
if [ ${FAILED_CHECKS} -eq 0 ]; then
    if [ ${WARNINGS} -eq 0 ]; then
        echo -e "${GREEN}✅ All health checks passed!${NC}"
        exit 0
    else
        echo -e "${YELLOW}⚠️  Health checks passed with ${WARNINGS} warning(s)${NC}"
        exit 0
    fi
else
    echo -e "${RED}❌ ${FAILED_CHECKS} health check(s) failed${NC}"
    exit 1
fi
