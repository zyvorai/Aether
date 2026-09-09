#!/usr/bin/env bash
# Migration dry-run E2E — validate demo specs, decide/explain, and dry-run migrate targets.
#
# Usage:
#   ./scripts/migration-dry-run-e2e.sh
#   AETHER_BIN=./target/release/aether make migration-dry-run-e2e
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

AETHER="${AETHER_BIN:-./target/release/aether}"
if [ ! -x "${AETHER}" ] || [ "$(wc -c < "${AETHER}" | tr -d ' ')" -lt 1000000 ]; then
  if [ -x "./target/debug/aether" ]; then
    AETHER="./target/debug/aether"
  fi
fi
if [ ! -x "${AETHER}" ]; then
  cargo build --release 2>/dev/null || cargo build
  AETHER="./target/release/aether"
  if [ ! -x "${AETHER}" ] || [ "$(wc -c < "${AETHER}" | tr -d ' ')" -lt 1000000 ]; then
    AETHER="./target/debug/aether"
  fi
fi
[ -x "${AETHER}" ] || { echo "Build aether first: cargo build" >&2; exit 1; }

declare -a SPECS=(
  "examples/demos/01-podman-to-k8s/workload-podman.yaml:kubernetes"
  "examples/demos/02-k8s-to-kubevirt/workload-kube.yaml:kubevirt"
)

echo "==> Migration dry-run E2E"

for entry in "${SPECS[@]}"; do
  spec="${entry%%:*}"
  target="${entry##*:}"
  name="$("$AETHER" --spec "$spec" validate 2>/dev/null | grep -E '^name:' | awk '{print $2}' || true)"
  if [ -z "$name" ]; then
    name="$(grep -E '^  name:' "$spec" | head -1 | awk '{print $2}')"
  fi
  echo "-- validate $spec ($name -> $target)"
  "$AETHER" --spec "$spec" validate
  echo "-- decide --explain $spec"
  "$AETHER" decide --spec "$spec" --explain || true
  echo "-- dry-run migrate $name -> $target"
  "$AETHER" --dry-run migrate "$name" "$target" --strategy blue-green
done

echo "Migration dry-run E2E complete."
