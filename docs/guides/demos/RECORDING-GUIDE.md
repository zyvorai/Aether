# Demo Recording Guide

> Capture CLI and dashboard footage for migration and decision-engine demos.

---

## Setup

1. Terminal: dark theme, large font (14–16pt)
2. Dashboard: `aether serve` → http://localhost:5090
3. Optional: second monitor for `AETHER_MIGRATION_TRACE=1` stderr

---

## CLI b-roll shots

| Shot | Command |
|------|---------|
| Product hook | `aether decide --spec examples/demos/01-podman-to-k8s/workload-podman.yaml --explain` |
| Migrate | `aether migrate demo-podman-k8s kubernetes --strategy blue-green --verbose-trace` |
| Validate | `aether validate --spec workload.yaml` |

---

## Dashboard b-roll

1. **Workloads** — open workload detail → Scoring tab
2. **AI Engine** — Recommendations tab → run recommend
3. **Intent Debugger** — per-runtime reasons panel
4. **Migrate modal** — strategy picker on Workloads page

---

## Demo scripts

Run packaged demos:

```bash
chmod +x scripts/run-demo-migration.sh
./scripts/run-demo-migration.sh 01
./scripts/run-demo-migration.sh 02
./scripts/run-demo-migration.sh 03
```

Record wall-clock time for `benchmarks/RESULTS.md`.

---

## Narration beats

1. One YAML spec
2. Explain why Kubernetes wins (or not)
3. Migrate without rewriting manifests
4. Show rollback / honest limits (no volume copy)

---

## Related

- [examples/demos/](../../examples/demos/)
- [MIGRATION-INTERNALS.md](../migration/MIGRATION-INTERNALS.md)
