# Aether Roadmap

> Ship vs Roadmap — no inflated claims.

---

## Shipped (Trust Layer)

- Product portability spine (README, PRODUCT.md)
- Migration internals + trace mode
- `aether decide --explain` + API/dashboard explain
- Deployment topologies, fleet doc (honest now vs later)
- Stateful portability guide
- Demo folders + bench harness template
- Security, networking, production, cloud matrix docs
- Ecosystem + product tiers

---

## Q3–Q4 targets (Roadmap)

| Item | Area | Tag |
|------|------|-----|
| **Hubble UI** | Observability | `observability` |
| **Cilium connectivity check** | Kubernetes | `kubernetes` |
| **PacketWolf bridge** | Ecosystem | `ecosystem` |
| Fleet overview dashboard | Fleet | `fleet` |
| Edge agent reconcile | Edge | `edge` |
| Cross-cluster volume replication | Migration | `migration` |
| Hosted SaaS control plane | Product | `hosted` |
| SBOM + signed images | Security | `security` |
| Anomaly-based placement | AI | `ai` |
| Federation engine | Fleet | `fleet` |

---

## How to read this doc

- **Ship** = in repo today with docs/tests
- **Roadmap** = architecture documented, not claimed in product UI
- **Lab** = `examples/labs/` only

Update this file when items graduate to Ship.

---

## Related engineering docs

- [NEXT-STEPS.md](NEXT-STEPS.md) — HA/OIDC implementation checklist
- [Trust Layer guides](index.md#trust-and-proof)
