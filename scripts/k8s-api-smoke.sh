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

echo "==> observability API smoke passed"
