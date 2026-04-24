#!/usr/bin/env bash
# ============================================================================
# health-check-all.sh — Validate Aether and common cluster dependencies
# ============================================================================
# Usage:
#   ./scripts/health-check-all.sh [namespace]
# ============================================================================

set -euo pipefail

NAMESPACE="${1:-${AETHER_NAMESPACE:-aether-system}}"
KUBECTL="${KUBECTL:-kubectl}"

TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

pass() { echo "  [PASS] $*"; }
warn() { echo "  [WARN] $*"; WARNINGS=$((WARNINGS + 1)); }
fail() { echo "  [FAIL] $*"; }
section() { echo ""; echo "== $*"; }

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

run_check() {
  TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
  if "$@"; then
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
  else
    FAILED_CHECKS=$((FAILED_CHECKS + 1))
  fi
}

check_cluster() {
  if ${KUBECTL} cluster-info >/dev/null 2>&1; then
    pass "Cluster is reachable"
    return 0
  fi
  fail "Cannot reach Kubernetes cluster"
  return 1
}

check_namespace() {
  if ${KUBECTL} get namespace "${NAMESPACE}" >/dev/null 2>&1; then
    pass "Namespace exists: ${NAMESPACE}"
    return 0
  fi
  fail "Namespace not found: ${NAMESPACE}"
  return 1
}

check_aether_deployment() {
  if ! ${KUBECTL} -n "${NAMESPACE}" get deployment aether >/dev/null 2>&1; then
    fail "Deployment/aether not found"
    return 1
  fi

  local ready desired available
  ready="$(${KUBECTL} -n "${NAMESPACE}" get deployment aether -o jsonpath='{.status.readyReplicas}')"
  desired="$(${KUBECTL} -n "${NAMESPACE}" get deployment aether -o jsonpath='{.status.replicas}')"
  available="$(${KUBECTL} -n "${NAMESPACE}" get deployment aether -o jsonpath='{.status.availableReplicas}')"
  ready="${ready:-0}"
  desired="${desired:-0}"
  available="${available:-0}"

  if [ "${ready}" -ge 1 ] && [ "${available}" -ge 1 ]; then
    pass "Aether deployment is ready (${ready}/${desired})"
    return 0
  fi

  fail "Aether deployment is not ready (${ready}/${desired})"
  return 1
}

check_aether_service() {
  if ${KUBECTL} -n "${NAMESPACE}" get svc aether >/dev/null 2>&1; then
    local node_port
    node_port="$(${KUBECTL} -n "${NAMESPACE}" get svc aether -o jsonpath='{.spec.ports[0].nodePort}')"
    pass "Aether service exists (nodePort=${node_port:-n/a})"
    return 0
  fi
  fail "Service/aether not found"
  return 1
}

check_pods() {
  local output crash_count pending_count failed_count
  output="$(${KUBECTL} -n "${NAMESPACE}" get pods --no-headers 2>/dev/null || true)"
  [ -n "${output}" ] || {
    fail "No pods found in namespace"
    return 1
  }

  pending_count="$(printf "%s\n" "${output}" | awk '$3 ~ /Pending|ContainerCreating/ {c++} END {print c+0}')"
  failed_count="$(printf "%s\n" "${output}" | awk '$3 ~ /Error|CrashLoopBackOff|ImagePullBackOff|ErrImagePull|Failed/ {c++} END {print c+0}')"
  crash_count="$(printf "%s\n" "${output}" | awk '$3 ~ /CrashLoopBackOff/ {c++} END {print c+0}')"

  if [ "${pending_count}" -gt 0 ]; then
    warn "Pending pods: ${pending_count}"
  fi
  if [ "${crash_count}" -gt 0 ]; then
    fail "CrashLoopBackOff pods: ${crash_count}"
    ${KUBECTL} -n "${NAMESPACE}" get pods | grep CrashLoopBackOff || true
    return 1
  fi
  if [ "${failed_count}" -gt 0 ]; then
    fail "Pods in failed/error states: ${failed_count}"
    ${KUBECTL} -n "${NAMESPACE}" get pods
    return 1
  fi

  pass "Pod states look healthy"
  return 0
}

check_endpoints() {
  local services missing
  services="$(${KUBECTL} -n "${NAMESPACE}" get svc --no-headers -o custom-columns=NAME:.metadata.name 2>/dev/null || true)"
  [ -n "${services}" ] || {
    warn "No services found in namespace"
    return 0
  }

  missing=0
  while read -r svc; do
    [ -n "${svc}" ] || continue
    local ready
    ready="$(${KUBECTL} -n "${NAMESPACE}" get endpoints "${svc}" -o jsonpath='{range .subsets[*].addresses[*]}x{end}' 2>/dev/null || true)"
    if [ -n "${ready}" ]; then
      pass "Service ${svc} has ready endpoints"
    else
      warn "Service ${svc} has no ready endpoints"
      missing=$((missing + 1))
    fi
  done <<< "${services}"

  [ "${missing}" -eq 0 ]
}

check_recent_events() {
  local warnings_output
  warnings_output="$(${KUBECTL} get events -n "${NAMESPACE}" --sort-by='.lastTimestamp' 2>/dev/null | tail -n 20 | grep -E 'Warning|Error' || true)"
  if [ -z "${warnings_output}" ]; then
    pass "No recent warning/error events"
    return 0
  fi
  warn "Recent warning/error events detected"
  echo "${warnings_output}"
  return 0
}

check_metrics_server() {
  if ! ${KUBECTL} top nodes >/dev/null 2>&1; then
    warn "metrics-server is unavailable; skipping kubectl top checks"
    return 0
  fi
  pass "metrics-server is available"
  ${KUBECTL} top nodes
  return 0
}

check_ingress() {
  local ingresses
  ingresses="$(${KUBECTL} -n "${NAMESPACE}" get ingress --no-headers 2>/dev/null || true)"
  if [ -z "${ingresses}" ]; then
    pass "No ingress resources found"
    return 0
  fi

  local failed=0
  while read -r line; do
    [ -n "${line}" ] || continue
    local name address
    name="$(printf "%s\n" "${line}" | awk '{print $1}')"
    address="$(printf "%s\n" "${line}" | awk '{print $4}')"
    if [ -n "${address}" ] && [ "${address}" != "<none>" ]; then
      pass "Ingress ${name} has address ${address}"
    else
      warn "Ingress ${name} has no external address yet"
      failed=$((failed + 1))
    fi
  done <<< "${ingresses}"

  [ "${failed}" -eq 0 ]
}

section "Cluster"
run_check check_cluster
run_check check_namespace

section "Aether"
run_check check_aether_deployment
run_check check_aether_service

section "Runtime"
run_check check_pods
run_check check_endpoints
run_check check_recent_events
run_check check_metrics_server
run_check check_ingress

echo ""
echo "Total Checks: ${TOTAL_CHECKS}"
echo "Passed:       ${PASSED_CHECKS}"
echo "Failed:       ${FAILED_CHECKS}"
echo "Warnings:     ${WARNINGS}"

if [ "${FAILED_CHECKS}" -gt 0 ]; then
  exit 1
fi
