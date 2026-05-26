# AI and Decision Engine Purpose

> What the "AI" layer actually does — and what is rule-based.

---

## Rename for accuracy

User-facing copy should say **Decision & placement engine** where the implementation is deterministic scoring (`ScoringEngine`), not ML inference.

Decorative "AI" labels remain on legacy API paths (`/api/ai/*`) for compatibility.

---

## Shipped capabilities (rule-based)

| Feature | Module | Purpose |
|---------|--------|---------|
| Runtime placement | `src/ai/scoring.rs` | Rank runtimes with reasons |
| Migration advice | `src/ai/migration.rs` | Strategy + timing hints |
| Cost estimates | `src/cost.rs` | Provider comparison |
| Drift detection | `src/drift.rs` | Spec vs runtime diff |
| Log analysis | `src/ai/analyzer.rs` | Pattern/heuristic log scan |
| Profiler | `src/ai/profiler.rs` | Right-sizing suggestions |
| Scaling advice | `src/ai/scaling.rs` | Replica forecast |

---

## Explainability (Ship)

- CLI: `aether decide --explain`
- API: `POST /api/ai/recommend` with `"explain": true`
- Dashboard: Intent Debugger per-runtime +/-

---

## Roadmap (not shipped)

| Feature | Notes |
|---------|-------|
| Anomaly detection | ML on metrics stream |
| Self-healing runtime pick | Closed-loop without operator |
| LLM manifest generation | Experimental only |

---

## Related

- [SCORING.md](SCORING.md)
- [EXAMPLES.md](EXAMPLES.md)
- [COST.md](../../COST.md)
