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
`AETHER_AUTO_ROTATE_SECRETS=1`, `AETHER_AUTO_ROLLBACK=1` (all default off;
`healing: auto` enables all four), and `AETHER_AUTO_SCALE=1` (reactive
autoscaling, default off).

### Health-gated auto-rollback

When `AETHER_AUTO_ROLLBACK=1`, the `serve` self-healing loop rolls a workload
back to its most recent snapshot when its circuit breaker opens — i.e. restarts
have been exhausted or a recovery attempt failed. It stops the failing instance
and redeploys from the last snapshot's spec, restoring state. A per-workload
cooldown (`AETHER_ROLLBACK_COOLDOWN_SECS`, default 600) prevents repeated
rollbacks to a persistently-bad snapshot, and each rollback is audited. Rollback
only fires on the circuit-open edge (not every cycle), so it complements
auto-restart rather than competing with it.

### Reactive autoscaling

When `AETHER_AUTO_SCALE=1`, `aether serve` runs a control loop that samples live
per-workload utilization from `metrics.k8s.io` (normalized against pod requests
— never synthetic), feeds a rolling history to the scaling engine, and applies a
replica change when warranted. It acts only on workloads that declare
`scaling.enabled`, stays within `scaling.min_replicas`/`max_replicas`, and
honors `scaling.cooldownSecs` between actions. Workloads without resource
requests, without metrics yet, or that aren't Deployments/StatefulSets are
skipped. Thresholds come from the `scaling` config
(`scale_up_threshold` / `scale_down_threshold`); sample interval via
`AETHER_SCHED_AUTOSCALE_SECS` (default 60). Each action emits a `ScalingEvent`
and is audited.

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

### TLS certificate lifecycle

The `serve` maintenance loop inspects the API TLS certificate on its schedule.
The ownership model mirrors secret rotation: a **self-signed** cert is one Aether
owns and can regenerate; a CA-issued cert is never self-signed and is only
alerted (renew it via cert-manager/ACME/your PKI). With
`AETHER_AUTO_RENEW_CERT=1`, an expiring self-signed cert is regenerated in place
via `openssl`, preserving its CN and SANs, valid for `AETHER_CERT_VALIDITY_DAYS`
(default 365). Regeneration takes effect on the **next serve start** (the live
TLS listener does not hot-reload), so it suits systemd/bare deployments; on
Kubernetes use cert-manager with a mounted Secret. Cert-check cadence:
`AETHER_SCHED_CERT_CHECK_SECS` (default 3600).

Copilot LLM: `OPENAI_API_KEY`, `AETHER_LLM_MODEL`, or `AETHER_OLLAMA_URL`

---

## Related

- [SCORING.md](SCORING.md)
- [EXAMPLES.md](EXAMPLES.md)
- [COST.md](../operations/COST.md)
