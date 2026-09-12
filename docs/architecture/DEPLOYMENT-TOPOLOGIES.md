---
hero:
  eyebrow: ARCHITECTURE
  title: Deployment Topologies
  tone: emerald
  lead: How to run Aether in dev, production HA, hybrid cloud, and edge scenarios.
---

See also: [Architecture](../architecture/ARCHITECTURE.md) · [Production Reference](../deployment/PRODUCTION-REFERENCE.md) · Helm chart `helm/aether/`

---

## Single-node (development)

```
Developer laptop / VM
├── aether serve (API :5090 + dashboard)
├── Podman workloads
└── ~/.aether/state.json
```

**Use when:** Local dev, POC, demos.  
**Limits:** No HA; state on local disk.

---

## Production HA

```
                    ┌─────────────┐
                    │  Ingress    │
                    │  (TLS)      │
                    └──────┬──────┘
           ┌───────────────┼───────────────┐
           ▼               ▼               ▼
    ┌────────────┐  ┌────────────┐  ┌────────────┐
    │ aether-api │  │ aether-api │  │ aether-api │
    │  replica   │  │  replica   │  │  replica   │
    └─────┬──────┘  └─────┬──────┘  └─────┬──────┘
          │               │               │
          └───────────────┼───────────────┘
                          ▼
              ┌───────────────────────┐
              │ PostgreSQL (state)    │
              │ Redis (sessions/SSE)  │
              └───────────────────────┘
                          │
                    ┌───────┴───────┐
                    ▼               ▼
               Kubernetes       KubeVirt
               target clusters  clusters
```

**Ship today:** Helm values for Postgres, Redis, OIDC/SAML, TLS — see [Enterprise security](../guides/security/ENTERPRISE-SECURITY.md) and [helm/aether](https://github.com/zyvorai/Aether/tree/main/helm/aether).

---

## Hybrid cloud

- **Control plane:** HA Aether in primary region
- **Workloads:** `runtime.allow` + kubeconfig contexts per cluster
- **GitOps:** Reconcile specs from repo to multiple targets

Operators use dashboard **Clusters** page for context inventory (kubeconfig-based).

---

## Edge (roadmap)

| Component | Status |
|-----------|--------|
| Central API + dashboard | **Ship** |
| Multi-cluster kubeconfig | **Ship** |
| Lightweight edge agent | **Roadmap** |
| Offline reconcile queue | **Roadmap** |

Document edge agents as target architecture only — not claimed as shipped.

---

## Topology selection

| Requirement | Topology |
|-------------|----------|
| Laptop demo | Single-node |
| Team staging | Single-node + shared K8s |
| Enterprise prod | HA + Postgres |
| Multi-region | HA + per-region kube contexts |
| Air-gapped | See [AIR-GAPPED.md](../deployment/AIR-GAPPED.md) |
