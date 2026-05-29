#!/usr/bin/env bash
# Smoke-test observability and Cilium API routes (server must be running).
set -euo pipefail

API="${AETHER_API:-${AETHER_API_BASE:-http://127.0.0.1:5090}}"

echo "==> GET /api/observability/summary"
code=$(curl -sS -o /tmp/aether-obs-summary.json -w '%{http_code}' "${API}/api/observability/summary")
[ "${code}" = "200" ] || { echo "FAIL: observability/summary HTTP ${code}" >&2; exit 1; }
python3 - <<'PY'
import json
with open("/tmp/aether-obs-summary.json") as f:
    body = json.load(f)
data = body.get("data") or body
assert "api_http_requests_total" in data, data
assert "prometheus_configured" in data
print("  ok: observability summary fields present")
PY

echo "==> GET /api/cluster/cilium/status"
code=$(curl -sS -o /tmp/aether-cilium-status.json -w '%{http_code}' "${API}/api/cluster/cilium/status")
if [ "${code}" = "200" ]; then
  python3 - <<'PY'
import json
with open("/tmp/aether-cilium-status.json") as f:
    body = json.load(f)
data = body.get("data") or body
for key in ("cni", "crds", "managed_policies", "metrics_server", "connectivity_check"):
    assert key in data, f"missing {key}: {data.keys()}"
print("  ok: cilium status fields present")
PY
elif [ "${code}" = "500" ]; then
  echo "  skip: no kubeconfig (500 expected in CI without cluster)"
else
  echo "FAIL: cilium/status HTTP ${code}" >&2
  exit 1
fi

echo "==> GET /api/cluster/cilium/hubble"
code=$(curl -sS -o /tmp/aether-hubble.json -w '%{http_code}' "${API}/api/cluster/cilium/hubble")
if [ "${code}" = "200" ]; then
  python3 - <<'PY'
import json
with open("/tmp/aether-hubble.json") as f:
    body = json.load(f)
data = body.get("data") or body
assert "url" in data and "source" in data, data
print("  ok: hubble discovery payload present")
PY
elif [ "${code}" = "500" ]; then
  echo "  skip: no kubeconfig (500 expected in CI without cluster)"
else
  echo "FAIL: cilium/hubble HTTP ${code}" >&2
  exit 1
fi

echo "==> GET /api/helm/catalog"
code=$(curl -sS -o /tmp/aether-helm-catalog.json -w '%{http_code}' "${API}/api/helm/catalog")
[ "${code}" = "200" ] || { echo "FAIL: helm/catalog HTTP ${code}" >&2; exit 1; }
python3 - <<'PY'
import json
with open("/tmp/aether-helm-catalog.json") as f:
    body = json.load(f)
charts = body.get("data") or body
assert isinstance(charts, list) and len(charts) >= 1, charts
assert "name" in charts[0], charts[0]
print(f"  ok: helm catalog has {len(charts)} charts")
PY

echo "==> POST /api/copilot/troubleshoot"
curl -sS -o /tmp/aether-workloads-smoke.json "${API}/api/workloads" || true
TROUBLESHOOT_PAYLOAD="$(python3 - <<'PY'
import json, os

api = os.environ.get("API", "http://127.0.0.1:5090")
try:
    with open("/tmp/aether-workloads-smoke.json") as f:
        body = json.load(f)
except Exception:
    body = {}
workloads = body.get("data") or []
cluster = next((w for w in workloads if w.get("source") == "cluster"), None)
if cluster:
    payload = {
        "workload": cluster.get("name", ""),
        "cluster": cluster.get("cluster"),
        "namespace": cluster.get("namespace"),
        "kind": cluster.get("kind"),
        "includeCopilotSummary": False,
    }
else:
    payload = {"workload": "web", "includeCopilotSummary": False}
print(json.dumps(payload))
PY
)"
code=$(curl -sS -o /tmp/aether-troubleshoot.json -w '%{http_code}' \
  -H 'Content-Type: application/json' \
  -d "${TROUBLESHOOT_PAYLOAD}" \
  "${API}/api/copilot/troubleshoot")
if [ "${code}" = "200" ]; then
  python3 - <<'PY'
import json
with open("/tmp/aether-troubleshoot.json") as f:
    body = json.load(f)
data = body.get("data") or body
for key in ("workload", "health_level", "recommendations"):
    assert key in data, f"missing {key}: {data.keys()}"
print("  ok: troubleshoot diagnosis payload present")
PY
elif [ "${code}" = "400" ]; then
  echo "  skip: no cluster/aether workload available for troubleshoot"
else
  echo "FAIL: copilot/troubleshoot HTTP ${code}" >&2
  exit 1
fi

echo "==> observability API smoke passed"

code=$(curl -sS -o /tmp/aether-pw-status.json -w '%{http_code}' "${API}/api/ecosystem/packetwolf/status")
if [ "${code}" = "200" ]; then
  python3 - <<'PY'
import json
with open("/tmp/aether-pw-status.json") as f:
    body = json.load(f)
data = body.get("data") or body
assert "configured" in data
print("  ok: packetwolf/status")
PY
else
  echo "FAIL: packetwolf/status HTTP ${code}" >&2
  exit 1
fi

code=$(curl -sS -o /tmp/aether-sbom.json -w '%{http_code}' "${API}/api/security/sbom")
if [ "${code}" = "200" ]; then
  echo "  ok: security/sbom"
else
  echo "FAIL: security/sbom HTTP ${code}" >&2
  exit 1
fi

code=$(curl -sS -o /tmp/aether-fleet-edge.json -w '%{http_code}' "${API}/api/fleet/edge/agents")
if [ "${code}" = "200" ]; then
  echo "  ok: fleet/edge/agents"
else
  echo "FAIL: fleet/edge/agents HTTP ${code}" >&2
  exit 1
fi

echo "==> four-pillars API smoke passed"
