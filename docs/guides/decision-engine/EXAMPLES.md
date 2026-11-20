---
hero:
  eyebrow: GUIDES
  title: Decision Engine Examples
  tone: violet
  lead: Sample aether decide --explain style traces for common workload shapes.
---

## ML inference (GPU, low latency)

**Spec hints:** `requirements.gpu: "1"`, `intent.goal: low-latency`

| Runtime | Typical outcome |
|---------|-----------------|
| KubeVirt | + GPU passthrough, + VM isolation |
| Kubernetes | − GPU scheduling complexity unless device plugin configured |
| Podman | − No production GPU path for large models |

**Recommended:** KubeVirt.

---

## Cost-optimized API

**Spec hints:** small CPU/memory, `intent.goal: cost-optimized`, `intent.budget.maxMonthlyUsd: 200`

| Runtime | Typical outcome |
|---------|-----------------|
| Podman | + Zero cloud spend for dev/staging |
| Kubernetes | + Efficient bin-packing at scale |
| KubeVirt | − VM overhead vs containers |

**Recommended:** Podman (local) or Kubernetes (shared cluster).

---

## HA web service

**Spec hints:** `scaling.minReplicas: 3`, health probes, `intent.resilience: High`

| Runtime | Typical outcome |
|---------|-----------------|
| Kubernetes | + Native HA, + HPA, + Intent HA bonus |
| KubeVirt | + Strong isolation, − Higher overhead |
| Podman | − Single-node unless orchestrated externally |

**Recommended:** Kubernetes.

---

## Stateful database (Postgres)

**Spec hints:** large storage, persistence block

| Runtime | Typical outcome |
|---------|-----------------|
| Kubernetes | + PVC, snapshots via backup module |
| KubeVirt | + VM + disk workflows |
| Podman | + Simple local dev |

**Note:** Scoring picks a runtime; **data migration is operator responsibility**. See [Stateful Portability](../migration/STATEFUL-PORTABILITY.md).

---

## Try it

```bash
aether decide --spec examples/workload-full-featured.yaml --explain
aether intent --spec examples/workload-full-featured.yaml
aether compare --spec examples/workload-full-featured.yaml
```
