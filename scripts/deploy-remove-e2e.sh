#!/usr/bin/env bash
# Deploy a minimal workload via API, verify it exists, delete it, verify removal.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

AETHER="${AETHER_BIN:-./target/release/aether}"
API="${AETHER_API:-http://127.0.0.1:5090}"
KEY="${AETHER_API_KEY:-}"

if ! curl -sf "${API}/health" >/dev/null 2>&1; then
  echo "Local API unavailable; trying lab default http://212.8.252.194:30090" >&2
  API="http://212.8.252.194:30090"
fi

if [ ! -x "${AETHER}" ] && [ -x "./target/debug/aether" ]; then
  AETHER="./target/debug/aether"
fi
[ -x "${AETHER}" ] || { echo "Build aether first: make release" >&2; exit 1; }

UNIQUE="e2e-rm-$(date +%s)"
SPEC="$(mktemp)"
trap 'rm -f "${SPEC}"' EXIT

cat > "${SPEC}" <<EOF
apiVersion: aether/v1
kind: Workload
metadata:
  name: ${UNIQUE}
  owner: e2e
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 100m
  memory: 128Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow:
    - kube
network:
  service: true
  ports:
    - containerPort: 80
      servicePort: 80
      protocol: TCP
EOF

auth=()
if [ -n "${KEY}" ]; then
  auth=(-H "Authorization: Bearer ${KEY}")
fi

curl_json() {
  curl -sf "${auth[@]}" "$@"
}

echo "==> Health"
curl_json "${API}/health" >/dev/null

echo "==> Deploy ${UNIQUE}"
curl_json -X POST "${API}/api/workloads" \
  -H 'Content-Type: application/json' \
  -d "$(python3 -c "import json, pathlib; print(json.dumps({'spec_yaml': pathlib.Path('${SPEC}').read_text()}))")" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert d.get('success'), d; print('  OK: deploy')"

echo "==> Verify listed"
curl_json "${API}/api/workloads" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); names=[w['name'] for w in d.get('data',[]) if isinstance(d.get('data'), list)]; assert '${UNIQUE}' in names, names; print('  OK: found in list')"

echo "==> Delete ${UNIQUE}"
curl_json -X DELETE "${API}/api/workloads/${UNIQUE}" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); assert d.get('success'), d; print('  OK: delete')"

echo "==> Verify removed"
curl_json "${API}/api/workloads" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); names=[w['name'] for w in d.get('data',[]) if isinstance(d.get('data'), list)]; assert '${UNIQUE}' not in names, 'still listed: ${UNIQUE}'; print('  OK: not in list')"

echo "Deploy/remove E2E passed."
