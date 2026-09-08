#!/usr/bin/env bash
# Validate example workload YAML files against Rust spec parsing and JSON Schema.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

AETHER="${AETHER_BIN:-./target/debug/aether}"

SPECS=(
  examples/demo-webserver.yaml
  examples/workload-full-featured.yaml
  examples/workload-k8s-advanced.yaml
  examples/labs/kubernetes/workload.yaml
  examples/labs/kubevirt/workload.yaml
)

echo "==> Rust spec validation (serde + business rules)"
for f in "${SPECS[@]}"; do
  echo "  $f"
  "$AETHER" --spec "$f" validate
done

if [[ -d web/dashboard/node_modules ]]; then
  echo "==> JSON Schema cross-check (ajv, kubernetes-focused examples)"
  (cd web/dashboard && npm run validate:schema)
else
  echo "  (skip ajv: npm ci in web/dashboard to enable JSON Schema cross-check)"
fi

echo "==> schema example validation passed"
