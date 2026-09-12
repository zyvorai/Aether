---
hero:
  eyebrow: USER STORIES
  title: Aether User Stories
  tone: sky
---

**Product:** Universal runtime portability — deploy once, run anywhere

Cross-reference: [Documentation index](https://github.com/zyvorai/Aether/blob/main/docs/README.md) · [Main README](https://github.com/zyvorai/Aether/blob/main/README.md)

## Personas

| Persona | Name | Focus |
|---------|------|-------|
| Platform Engineer | Alex | Migrate workloads across Podman, K8s, KubeVirt |
| DevOps Lead | Morgan | Blue-green and rolling migrations with zero downtime |
| SRE | Jordan | Drift detection, health checks, and auto-reconciliation |
| Developer | Riley | Single YAML spec for local Podman and prod K8s |

---

### Story 1 — Deploy one YAML to any runtime

**As Alex** (Platform Engineer), I want deploy a workload yaml to podman, kubernetes, or kubevirt from one spec, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | aether deploy, runtime selection, intent engine |

---

### Story 2 — Migrate K8s to KubeVirt

**As Morgan** (DevOps Lead), I want migrate a running workload from kubernetes to kubevirt with blue-green strategy, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | migration engine, 16 runtime pairs, rollback |

---

### Story 3 — Detect configuration drift

**As Jordan** (SRE), I want see when live workloads diverge from declared spec and reconcile automatically, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | drift detection, orchestrate reconcile |

---

### Story 4 — Score runtime with intent

**As Alex** (Platform Engineer), I want declare intent (low-latency, cost-optimized) and get scored runtime recommendations, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | intent engine, radar chart in UI |

---

### Story 5 — Monitor fleet in TUI or web

**As Jordan** (SRE), I want watch all workloads across runtimes with sse updates and log tail, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | React dashboard, TUI, useEventStream |

---

### Story 6 — Enforce deployment policy

**As Morgan** (DevOps Lead), I want block non-compliant deploys in production via policy gate, **so that** I deliver reliable outcomes.

| Criterion | Notes |
|-----------|-------|
| Core capability | aether policy-check, production policy sets |

---

## Validation

Map each story to smoke tests, CI jobs, or manual lab steps before marking production-ready.
