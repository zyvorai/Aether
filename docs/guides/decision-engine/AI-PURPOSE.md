# AI and Decision Engine Purpose

> What the "AI" layer actually does — and what is rule-based vs learned vs LLM-assisted.

---

## Shipped capabilities

| Feature | Module | Purpose |
|---------|--------|---------|
| Runtime placement | `src/ai/scoring.rs` | Rank runtimes with reasons (+ learned `RuntimeHistory`) |
| Migration advice | `src/ai/migration.rs` | Strategy + predictive plan (`MigrationPlanProposal`) |
| Cost estimates | `src/cost.rs` | Provider comparison |
| FinOps intelligence | `src/intelligence/finops.rs` | Utilization-aware optimization recommendations |
| Drift detection | `src/drift.rs` | Spec vs runtime diff (+ optional auto-reconcile) |
| Log analysis | `src/ai/analyzer.rs` | Pattern/heuristic log scan |
| Profiler | `src/ai/profiler.rs` | Right-sizing suggestions |
| Scaling advice | `src/ai/scaling.rs` | Replica forecast (Prometheus when configured) |
| Affinity learning | `src/ai/affinity.rs` | Deployment outcome matrix (wired on deploy/migrate) |
| Failure prediction | `src/intelligence/predict.rs` | Classical risk scoring from health + spec |
| Ops Copilot | `src/copilot/` | NL interface with tool calling (OpenAI/Ollama/rule fallback) |
| Runtime evolution | `src/intelligence/evolution.rs` | Continuous runtime trajectory recommendations |
| Self-healing | `src/intelligence/healer.rs` | Autonomous restart + drift reconcile (opt-in) |

---

## API endpoints (intelligence layer)

- `GET /api/context/snapshot` — unified platform context
- `GET /api/command-center/briefing` — Command Center narrative (health, savings, migrations, capacity)
- `GET /api/intelligence/predictions` — fleet failure risk
- `GET /api/intelligence/cost-optimize` — FinOps recommendations
- `GET /api/intelligence/evolution/status` — runtime evolution status
- `POST /api/copilot/chat` — AI ops copilot
- `GET /api/ai/migration-plan/:name/:target` — predictive migration plan

---

## Autonomy

Workloads may declare `autonomy` in spec:

```yaml
autonomy:
  migration: recommend | auto-low-risk | auto
  healing: recommend | auto-low-risk | auto
  evolution: recommend | auto-low-risk | auto
```

Environment: `AETHER_AUTO_RESTART=1`, `AETHER_AUTO_RECONCILE=1`,
`AETHER_AUTO_ROTATE_SECRETS=1` (all default off; `healing: auto` enables all three).

### Autonomous secret rotation

When `AETHER_AUTO_ROTATE_SECRETS=1`, the `serve` maintenance loop rotates due
secrets **that Aether owns** to a freshly-generated CSPRNG credential. A secret
is owned only when its rotation policy sets `generate: true`:

```yaml
rotation_policy:
  interval_days: 90
  max_age_days: 365
  notify_before_days: 14
  generate: true   # Aether owns this value → auto-generate on rotation
```

Secrets with `generate: false` (the default) mirror an externally-managed
credential and are **never overwritten** — they are only alerted (event
`Secret rotation deferred`) so an operator/external system rotates them. This
prevents auto-rotation from clobbering a value that must match an external
system. Each autonomous rotation is recorded in the audit trail.

Copilot LLM: `OPENAI_API_KEY`, `AETHER_LLM_MODEL`, or `AETHER_OLLAMA_URL`

---

## Related

- [SCORING.md](SCORING.md)
- [EXAMPLES.md](EXAMPLES.md)
- [COST.md](../../COST.md)
