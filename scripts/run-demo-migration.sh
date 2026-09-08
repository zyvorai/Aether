#!/usr/bin/env bash
# Run a numbered migration demo and capture timings for benchmarks/RESULTS.md
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEMO="${1:-}"

if [[ -z "$DEMO" ]]; then
  echo "Usage: $0 <01|02>" >&2
  exit 1
fi

case "$DEMO" in
  01)
    DIR="$ROOT/examples/demos/01-podman-to-k8s"
    SPEC="$DIR/workload-podman.yaml"
    NAME="demo-podman-k8s"
    TARGET="kubernetes"
    ;;
  02)
    DIR="$ROOT/examples/demos/02-k8s-to-kubevirt"
    SPEC="$DIR/workload-kube.yaml"
    NAME="demo-k8s-kubevirt"
    TARGET="kubevirt"
    ;;
  *)
    echo "Unknown demo: $DEMO" >&2
    exit 1
    ;;
esac

AETHER="${AETHER_BIN:-$ROOT/target/release/aether}"

if [[ ! -x "$AETHER" ]]; then
  echo "Building aether..."
  (cd "$ROOT" && cargo build --release)
fi

echo "==> Validate $SPEC"
"$AETHER" validate --spec "$SPEC"

echo "==> Decide (explain)"
"$AETHER" decide --spec "$SPEC" --explain || true

START=$(date +%s.%N)
echo "==> Migrate $NAME -> $TARGET"
if "$AETHER" migrate "$NAME" "$TARGET" --strategy blue-green --verbose-trace 2>/dev/null; then
  END=$(date +%s.%N)
  DURATION=$(echo "$END - $START" | bc)
  echo "MIGRATION_DURATION_SEC=$DURATION"
  echo "Append to benchmarks/RESULTS.md: demo-$DEMO migrate ${DURATION}s"
else
  echo "Migrate skipped or failed (runtime may be unavailable in this environment)"
  exit 0
fi
