#!/usr/bin/env bash
# Import grafana/dashboard.json into Grafana via HTTP API.
#
# Required env:
#   GRAFANA_URL          e.g. https://grafana.example.com
#   GRAFANA_API_KEY      or GRAFANA_USER + GRAFANA_PASSWORD
#
# Optional:
#   GRAFANA_FOLDER_UID   target folder
#   AETHER_GRAFANA_DASHBOARD_UID  printed hint for API server env
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DASHBOARD_JSON="${REPO_DIR}/grafana/dashboard.json"

GRAFANA_URL="${GRAFANA_URL:-${AETHER_GRAFANA_URL:-}}"
[[ -n "${GRAFANA_URL}" ]] || { echo "Set GRAFANA_URL or AETHER_GRAFANA_URL" >&2; exit 1; }
GRAFANA_URL="${GRAFANA_URL%/}"

AUTH=()
if [[ -n "${GRAFANA_API_KEY:-}" ]]; then
  AUTH=(-H "Authorization: Bearer ${GRAFANA_API_KEY}")
elif [[ -n "${GRAFANA_USER:-}" && -n "${GRAFANA_PASSWORD:-}" ]]; then
  AUTH=(-u "${GRAFANA_USER}:${GRAFANA_PASSWORD}")
else
  echo "Set GRAFANA_API_KEY or GRAFANA_USER/GRAFANA_PASSWORD" >&2
  exit 1
fi

payload=$(REPO_DIR="${REPO_DIR}" GRAFANA_FOLDER_UID="${GRAFANA_FOLDER_UID:-}" python3 - <<'PY'
import json, os, sys
path = os.path.join(os.environ["REPO_DIR"], "grafana", "dashboard.json")
with open(path) as f:
    dash = json.load(f)
# Grafana import expects dashboard wrapper with overwrite
out = {"dashboard": dash, "overwrite": True}
if os.environ.get("GRAFANA_FOLDER_UID"):
    out["folderUid"] = os.environ["GRAFANA_FOLDER_UID"]
print(json.dumps(out))
PY
)

curl -fsS "${AUTH[@]}" -H 'Content-Type: application/json' \
  -X POST "${GRAFANA_URL}/api/dashboards/db" \
  -d "${payload}" | python3 -c '
import json, sys
r = json.load(sys.stdin)
uid = r.get("uid") or (r.get("dashboard") or {}).get("uid")
print("Imported dashboard uid:", uid)
if uid:
    print("Set on API server: export AETHER_GRAFANA_DASHBOARD_UID=" + uid)
'
