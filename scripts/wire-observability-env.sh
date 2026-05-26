#!/usr/bin/env bash
# Discover in-cluster Prometheus/Grafana and print Aether API env exports.
#
# Usage:
#   ./scripts/wire-observability-env.sh
#   eval "$(./scripts/wire-observability-env.sh)"
#
# Optional:
#   NAMESPACE=aether-system
#   GRAFANA_API_KEY=...  — run import-grafana-dashboard.sh when Grafana found
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
NS="${AETHER_NAMESPACE:-aether-system}"

if ! command -v kubectl >/dev/null 2>&1; then
  echo "# kubectl not found" >&2
  exit 1
fi

prom_url=""
grafana_url=""

# Common Prometheus Service patterns (kube-prometheus-stack, legacy prometheus-server)
while IFS= read -r line; do
  [ -n "${line}" ] || continue
  prom_url="${line}"
  break
done < <(
  kubectl get svc -A -o json 2>/dev/null | python3 - <<'PY' || true
import json, sys
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
candidates = []
for item in data.get("items", []):
    name = item["metadata"]["name"]
    ns = item["metadata"]["namespace"]
    labels = item["metadata"].get("labels") or {}
    if "prometheus" not in name.lower() and labels.get("app.kubernetes.io/name") != "prometheus":
        continue
    spec = item.get("spec") or {}
    ports = spec.get("ports") or []
    port = next((p.get("port") for p in ports if p.get("name") in ("http-web", "web", "http")), ports[0]["port"] if ports else 9090)
    candidates.append((ns, name, port))
for ns, name, port in sorted(candidates, key=lambda x: (0 if x[0] == "monitoring" else 1, x[1])):
    print(f"http://{name}.{ns}.svc:9090" if port == 9090 else f"http://{name}.{ns}.svc:{port}")
    break
PY
)

while IFS= read -r line; do
  [ -n "${line}" ] || continue
  grafana_url="${line}"
  break
done < <(
  kubectl get svc -A -o json 2>/dev/null | python3 - <<'PY' || true
import json, sys
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
for item in data.get("items", []):
    name = item["metadata"]["name"]
    ns = item["metadata"]["namespace"]
    if "grafana" not in name.lower():
        continue
    spec = item.get("spec") or {}
    ports = spec.get("ports") or []
    port = next((p.get("port") for p in ports if p.get("name") in ("service", "http", "grafana")), ports[0]["port"] if ports else 80)
    print(f"http://{name}.{ns}.svc:{port}")
    break
PY
)

if [ -n "${prom_url}" ]; then
  echo "export AETHER_PROMETHEUS_URL=${prom_url}"
fi
if [ -n "${grafana_url}" ]; then
  echo "export AETHER_GRAFANA_URL=${grafana_url}"
fi

if [ -n "${grafana_url}" ] && [ -n "${GRAFANA_API_KEY:-}" ]; then
  if GRAFANA_URL="${grafana_url}" GRAFANA_API_KEY="${GRAFANA_API_KEY}" "${SCRIPT_DIR}/import-grafana-dashboard.sh" 2>/dev/null | tee /tmp/aether-grafana-import.log; then
    uid=$(grep -o 'AETHER_GRAFANA_DASHBOARD_UID=[^ ]*' /tmp/aether-grafana-import.log | tail -1 | cut -d= -f2 || true)
    [ -n "${uid}" ] && echo "export AETHER_GRAFANA_DASHBOARD_UID=${uid}"
  fi
fi

if [ -z "${prom_url}" ] && [ -z "${grafana_url}" ]; then
  echo "# No Prometheus or Grafana Service found in cluster (install monitoring stack first)" >&2
fi

echo "# Patch Aether deployment (example):"
echo "# kubectl -n ${NS} set env deployment/aether AETHER_PROMETHEUS_URL=\${AETHER_PROMETHEUS_URL:-} AETHER_GRAFANA_URL=\${AETHER_GRAFANA_URL:-}"
