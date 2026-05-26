#!/usr/bin/env bash
# Aether-specific benchmark harness: deploy latency, API throughput, migrate duration
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AETHER="${AETHER_BIN:-$ROOT/target/release/aether}"
API="${AETHER_API:-http://127.0.0.1:5090}"
SPEC="${AETHER_BENCH_SPEC:-$ROOT/examples/demo-webserver.yaml}"
OUT="${AETHER_BENCH_OUT:-$ROOT/benchmarks/RESULTS.md}"

if [[ ! -x "$AETHER" ]]; then
  (cd "$ROOT" && cargo build --release)
fi

ts() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

echo "==> validate latency"
START=$(python3 - <<'PY'
import time; print(time.time())
PY
)
"$AETHER" validate --spec "$SPEC" >/dev/null
END=$(python3 - <<'PY'
import time; print(time.time())
PY
)
VALIDATE_MS=$(python3 - <<PY
print(int((${END} - ${START}) * 1000))
PY
)

echo "==> decide latency"
START=$(python3 - <<'PY'
import time; print(time.time())
PY
)
"$AETHER" decide --spec "$SPEC" --explain >/dev/null
END=$(python3 - <<'PY'
import time; print(time.time())
PY
)
DECIDE_MS=$(python3 - <<PY
print(int((${END} - ${START}) * 1000))
PY
)

API_OK="n/a"
API_MS="n/a"
if curl -sf "$API/api/health" >/dev/null 2>&1; then
  START=$(python3 - <<'PY'
import time; print(time.time())
PY
)
  curl -sf "$API/api/workloads" >/dev/null
  END=$(python3 - <<'PY'
import time; print(time.time())
PY
)
  API_MS=$(python3 - <<PY
print(int((${END} - ${START}) * 1000))
PY
)
  API_OK="yes"
fi

{
  echo ""
  echo "## Run $(ts)"
  echo ""
  echo "| Metric | Value |"
  echo "|--------|-------|"
  echo "| validate (ms) | $VALIDATE_MS |"
  echo "| decide --explain (ms) | $DECIDE_MS |"
  echo "| GET /api/workloads (ms) | $API_MS (server=$API_OK) |"
} >> "$OUT"

echo "Results appended to $OUT"
