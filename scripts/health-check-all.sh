#!/usr/bin/env bash
# ============================================================================
# health-check-all.sh — Validate Aether and common cluster dependencies
# ============================================================================
# Usage:
#   ./scripts/health-check-all.sh [namespace]
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"

NAMESPACE="${1:-${AETHER_NAMESPACE:-aether-system}}"
KUBECTL="${KUBECTL:-kubectl}"

TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0
WARNINGS=0

pass() { aether_ok "$*"; }
warn() { aether_warn "$*"; WARNINGS=$((WARNINGS + 1)); }
fail() { aether_bad "$*"; }
section() { aether_section "$*"; }

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

  if ${KUBECTL} create namespace "${NAMESPACE}" >/dev/null 2>&1; then
    warn "Namespace was missing and has been created: ${NAMESPACE}"
    return 0
  fi

  fail "Namespace not found and could not be created: ${NAMESPACE}"
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

# Informational only (never fails the ritual)
check_mesh_kubevirt() {
  if ${KUBECTL} get crd virtualmachines.kubevirt.io &>/dev/null; then
    pass "KubeVirt API detected (virtualmachines.kubevirt.io)"
  else
    warn "KubeVirt CRDs not found (VM workloads need KubeVirt on the cluster)"
  fi
  return 0
}

check_mesh_metal3() {
  if ${KUBECTL} get crd baremetalhosts.metal3.io &>/dev/null; then
    pass "Metal3 API detected (baremetalhosts.metal3.io)"
  else
    warn "Metal3 CRDs not found (optional unless you provision bare metal)"
  fi
  return 0
}

check_mesh_cilium() {
  if ${KUBECTL} get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
    pass "CiliumNetworkPolicy CRD detected (deploy scripts can install aether egress allow)"
  else
    warn "Cilium CRDs not found (fine if the cluster uses another CNI)"
  fi
  return 0
}

aether_banner_health
aether_kv "Namespace" "${NAMESPACE}"
aether_kv "kubectl" "${KUBECTL}"
echo ""

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

section "Cluster mesh (optional)"
run_check check_mesh_kubevirt
run_check check_mesh_metal3
run_check check_mesh_cilium

echo ""
echo -e "${A_BLD}  ── Ritual tally ──${A_RST}"
aether_kv "Checks run" "${TOTAL_CHECKS}"
aether_kv "Blessed ✓" "${PASSED_CHECKS}"
aether_kv "Cursed ✗" "${FAILED_CHECKS}"
aether_kv "Omens ⚡" "${WARNINGS}"

if [ "${FAILED_CHECKS}" -gt 0 ]; then
  aether_finale_health_bad
  exit 1
fi

aether_finale_health_ok
exit 0
