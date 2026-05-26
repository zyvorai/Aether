# Scoring Engine

> How Aether ranks runtimes — weights, intent multipliers, and explain output.

---

## Formula

Each allowed runtime receives four dimension scores (0–1):

| Dimension | Default weight | What it measures |
|-----------|----------------|------------------|
| Cost | 0.25 | Resource size vs runtime economics |
| Performance | 0.25 | CPU/memory/GPU fit |
| Reliability | 0.25 | Health, restart, isolation |
| Availability | 0.25 | HA, scaling, runtime uptime profile |

**Total score** = weighted sum × intent multipliers × workload-class adjustments.

Weights are configurable via `EngineConfig` / `~/.aether/config.toml`.

---

## Workload classification

The engine classifies specs before scoring:

| Class | Triggers |
|-------|----------|
| Stateless | Default web/API |
| Stateful | PVC / persistence block |
| GpuCompute | `requirements.gpu` set |
| BareMetal | High CPU/memory thresholds |
| Batch | Cron/batch patterns |
| General | Fallback |

Classification filters ineligible runtimes (e.g. GPU → KubeVirt/Metal3 bias).

---

## Intent multipliers

When `intent:` is present in the spec:

- **Goal** (`low-latency`, `cost-optimized`, `high-throughput`, `balanced`) shifts weights
- **SLA** latency/availability thresholds filter or penalize runtimes
- **Budget** caps favor lower-cost runtimes
- **Resilience** `High` boosts Kubernetes HA
- **Compliance** isolation/encryption flags favor KubeVirt/K8s

Run `aether intent --spec workload.yaml` to compare with and without intent.

---

## Explain mode

### CLI

```bash
aether decide --spec examples/workload.yaml --explain
```

Prints score table plus **per-runtime** `+` reasons and `-` warnings.

### API

```bash
curl -X POST http://localhost:5090/api/ai/recommend \
  -H "Content-Type: application/json" \
  -d '{"yaml":"<workload yaml>", "explain": true}'
```

### Dashboard

AI Engine page and Intent Debugger call `explain: true` and show all runtimes.

---

## Confidence

Confidence reflects score separation between first and second ranked runtimes. Close scores → lower confidence; update spec constraints or intent for a clearer winner.

---

## Related

- [Examples](EXAMPLES.md) — sample traces
- [AI Purpose](AI-PURPOSE.md) — what is rule-based vs roadmap
- Code: `src/ai/scoring.rs`, `src/engine.rs`
