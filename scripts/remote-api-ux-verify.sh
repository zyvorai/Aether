#!/usr/bin/env bash
# Validate dashboard-facing APIs on a remote Aether instance and cross-check UX data consistency.
# Usage: AETHER_API=http://212.8.252.194:30090 ./scripts/remote-api-ux-verify.sh
set -euo pipefail

API="${AETHER_API:-${AETHER_API_BASE:-http://127.0.0.1:5090}}"
AUTH=()
if [ -n "${AETHER_API_KEY:-}" ]; then
  AUTH=(-H "Authorization: Bearer ${AETHER_API_KEY}")
fi

TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

pass=0
fail=0
warn=0

ok() { echo "  ✓ $*"; pass=$((pass + 1)); }
bad() { echo "  ✗ $*" >&2; fail=$((fail + 1)); }
note() { echo "  ⚡ $*"; warn=$((warn + 1)); }

fetch() {
  local path="$1"
  local out="${TMP}/$(echo "$path" | tr '/?&=' '_')"
  local code
  code="$(curl -sS -m 20 "${AUTH[@]}" -o "${out}" -w '%{http_code}' "${API}${path}")"
  echo "${code}" >"${out}.code"
  printf '%s' "${out}"
}

expect_code() {
  local path="$1" want="$2"
  local f code
  f="$(fetch "$path")"
  code="$(cat "${f}.code")"
  if [ "${code}" = "${want}" ]; then
    ok "${path} → ${code}"
  else
    bad "${path} → ${code} (expected ${want})"
  fi
}

echo "==> Remote API + UX consistency → ${API}"
echo ""

section() { echo ""; echo "── $1 ──"; }

section "Core"
expect_code /health 200
expect_code /api/server 200
expect_code /api/system/ready 200
expect_code /api/auth/providers 200
expect_code /api/auth/me 200
expect_code /api/dashboard/version 200

section "Workloads & cluster (dashboard pages)"
expect_code /api/workloads 200
expect_code /api/cluster/summary 200
expect_code /api/cluster/namespaces?cluster=active-client 200
expect_code "/api/cluster/metrics/summary?cluster=active-client&namespace=aether-system" 200
expect_code /api/cluster/cilium/status 200
expect_code "/api/cluster/browse?cluster=active-client&namespace=aether-system&kind=CiliumNetworkPolicy" 200
expect_code "/api/cluster/browse?cluster=active-client&namespace=_cluster&kind=CiliumClusterwideNetworkPolicy" 200

section "Observability & platform"
expect_code /api/observability/summary 200
expect_code /api/metrics 200
expect_code /api/platform/recommendations 200

section "Overview / ops pages"
for p in \
  /api/events/summary \
  /api/events \
  /api/orchestrator/summary \
  /api/orchestrator/status \
  /api/backups \
  /api/secrets \
  /api/plugins \
  /api/environments \
  /api/rbac/keys \
  /api/audit \
  /api/audit/verify \
  /api/gitops/status \
  /api/templates \
  /api/scheduler/utilization \
  /api/scheduler/optimize \
  /api/alerts/status \
  /api/cost/chargeback?provider=aws; do
  expect_code "${p}" 200
done

section "Confidential fabric"
for p in \
  /api/confidential/capabilities \
  /api/confidential/fleet \
  /api/confidential/trust-score \
  /api/confidential/intelligence \
  /api/confidential/kata/status \
  /api/confidential/sovereign/status \
  /api/confidential/images; do
  expect_code "${p}" 200
done

section "Cross-check UX data consistency"
python3 - "${TMP}" "${API}" <<'PY'
import json, sys, glob, os

tmp = sys.argv[1]
api = sys.argv[2]

def load(path_suffix):
    for f in glob.glob(os.path.join(tmp, "*" + path_suffix)):
        with open(f) as fh:
            return json.load(fh)
    return None

def data(body):
    if not body:
        return {}
    return body.get("data", body)

workloads_body = load("_api_workloads")
summary_body = load("_api_cluster_summary")
cilium_body = load("_api_cluster_cilium_status")
obs_body = load("_api_observability_summary")
server_body = load("_api_server")

workloads = data(workloads_body)
if isinstance(workloads, dict):
    workloads = workloads.get("items") or workloads.get("workloads") or []
summary = data(summary_body)
cilium = data(cilium_body)
obs = data(obs_body)
server = data(server_body)

errors = []
warnings = []
cluster_count = 0

if not isinstance(workloads, list):
    errors.append(f"workloads payload not a list: {type(workloads)}")
else:
    cluster_count = sum(1 for w in workloads if w.get("source") == "cluster")
    aether_count = sum(1 for w in workloads if w.get("source") == "aether")
    print(f"  workloads: total={len(workloads)} cluster={cluster_count} aether={aether_count}")
    if cluster_count + aether_count > len(workloads):
        warnings.append("workloads source counts exceed total")

if summary:
    cc = summary.get("cluster_count", 0)
    hc = summary.get("healthy_clusters", 0)
    wc = summary.get("workload_count", 0)
    connected = summary.get("connected")
    print(f"  cluster/summary: connected={connected} clusters={cc} healthy={hc} workload_count={wc}")
    if connected and hc < 1:
        errors.append("cluster summary connected but no healthy clusters")
    if isinstance(workloads, list) and wc and abs(wc - cluster_count) > 0:
        errors.append(
            f"workload_count ({wc}) differs from cluster-discovered /api/workloads ({cluster_count})"
        )

if cilium:
    cni = cilium.get("cni")
    ds = cilium.get("cilium_daemonset_ready")
    conn = cilium.get("connectivity_check")
    ms = cilium.get("metrics_server")
    print(f"  cilium/status: cni={cni} ds_ready={ds} connectivity={conn} metrics_server={ms}")
    if cni not in ("cilium", "other", "unknown"):
        errors.append(f"unexpected cni value: {cni}")
    if cni == "cilium" and not ds:
        warnings.append("cni=cilium but daemonset not ready")

server_cilium = (server.get("kubernetes") or {}).get("cilium") if server else None
if server_cilium and cilium:
    if server_cilium.get("cni") != cilium.get("cni"):
        errors.append("server.kubernetes.cilium.cni != /api/cluster/cilium/status cni")
    else:
        print("  server.kubernetes.cilium matches cilium/status")

obs_cilium = obs.get("cilium") if obs else None
if obs_cilium and cilium:
    if obs_cilium.get("cni") != cilium.get("cni"):
        errors.append("observability.cilium.cni != cilium/status cni")
    else:
        print("  observability.cilium matches cilium/status")

if obs and "prometheus_configured" not in obs:
    errors.append("observability summary missing prometheus_configured")

for w in warnings:
    print(f"WARN: {w}")
for e in errors:
    print(f"FAIL: {e}")
    sys.exit(1)
print("  cross-checks passed")
PY
if [ $? -eq 0 ]; then ok "UX data consistency"; else bad "UX data consistency"; fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Summary: ${pass} passed, ${fail} failed, ${warn} warnings"
if [ "${fail}" -gt 0 ]; then
  exit 1
fi
