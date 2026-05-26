# Aether Benchmark Results

Baseline metrics from `benchmarks/aether-bench.sh` and demo migrations. Numbers vary by hardware — publish honest modest baselines.

---

## Environment template

| Field | Example |
|-------|---------|
| Host | Apple M2 / 16Gi |
| Aether version | git SHA |
| API | local `aether serve` |
| K8s | kind 0.22 |

---

## CLI baselines (reference)

| Metric | Typical range |
|--------|---------------|
| `aether validate` | 5–50 ms |
| `aether decide --explain` | 10–80 ms |
| `aether migrate` (blue-green) | 30s–5m (runtime-dependent) |

---

## Demo migration timings

| Demo | Path | Duration | Notes |
|------|------|----------|-------|
| 01 | Podman → K8s | _run `./scripts/run-demo-migration.sh 01`_ | |
| 02 | K8s → KubeVirt | _run script_ | Requires KubeVirt |
| 03 | KubeVirt → Metal3 | _run script_ | Lab only |

---

## API throughput

Run with server up:

```bash
./benchmarks/aether-bench.sh
```

Or use k6 against `/api/workloads` — see [README.md](README.md).

---

## CI

Optional manual workflow: `.github/workflows/` — dispatch `aether-bench.sh` on release tags.

---

_Last updated: 2026-05-26 — local benchmark run via `benchmarks/aether-bench.sh`._

## Run 2026-05-26T09:59:06Z

| Metric | Value |
|--------|-------|
| validate (ms) | 56 |
| decide --explain (ms) | 48 |
| GET /api/workloads (ms) | n/a (server=n/a) |
