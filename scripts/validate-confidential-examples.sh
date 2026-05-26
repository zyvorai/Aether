#!/usr/bin/env bash
# Validate all examples/confidential-*.yaml specs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

AETHER="${AETHER_BIN:-./target/release/aether}"
if [ ! -x "${AETHER}" ]; then
  AETHER="./target/debug/aether"
fi
[ -x "${AETHER}" ] || { echo "Build aether first: make release" >&2; exit 1; }

shopt -s nullglob
specs=(examples/confidential-*.yaml)
if [ ${#specs[@]} -eq 0 ]; then
  echo "No examples/confidential-*.yaml found"
  exit 0
fi

for spec in "${specs[@]}"; do
  if ! grep -qE '^kind:[[:space:]]+Workload' "${spec}" 2>/dev/null; then
    echo "==> skip ${spec} (Kubernetes manifest, not aether/v1 Workload)"
    continue
  fi
  echo "==> validate ${spec}"
  "${AETHER}" --spec "${spec}" validate
done

echo "All confidential examples valid."
