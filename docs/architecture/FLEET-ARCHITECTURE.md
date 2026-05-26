# Fleet Architecture

> Multi-cluster inventory today and federation roadmap.

---

## Now (shipped)

| Capability | Description |
|------------|-------------|
| Kubeconfig contexts | Dashboard **Clusters** page lists contexts from merged kubeconfig |
| Per-workload runtime | Each workload bound to one runtime + cluster context |
| GitOps reconcile | Push spec → Aether applies to configured target |
| Policy (OPA) | Central policy check before deploy/migrate |
| Audit/events | Central log of mutations across workloads |

**Model:** One Aether control plane, many cluster **contexts** — not a separate fleet controller per cluster.

---

## Roadmap

| Capability | Target |
|------------|--------|
| Fleet overview dashboard | Read-only aggregate of `/api/clusters` |
| Policy propagation | GitOps + OPA bundles to agents |
| Federation engine | Central placement across clusters |
| Edge agents | Lightweight reconcile at edge sites |

Issues tagged `fleet` in [ROADMAP.md](../ROADMAP.md).

---

## Policy propagation (planned path)

```
Git repo (specs + policies)
        │
        ▼
Aether GitOps loop ──► OPA evaluate ──► target cluster apply
```

Today: single control plane evaluates policy; federation of policy bundles is **Roadmap**.

---

## Comparison

| Approach | Aether now | Full federation (roadmap) |
|----------|------------|----------------------------|
| Inventory | Kubeconfig contexts | Agent-reported fleet registry |
| Deploy | Per-context API/CLI | Policy-driven batch placement |
| Migrate | Per workload | Coordinated fleet migrations |
| Drift | Per workload | Fleet-wide drift dashboard |

---

## Related

- [Deployment Topologies](DEPLOYMENT-TOPOLOGIES.md)
- Dashboard: `ClustersPage.tsx`
- API: `/api/clusters`
