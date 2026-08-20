# Zyvor Platform Ecosystem

> Where Aether fits alongside HyperSDK suite products.

---

## Role diagram

```
                    ┌─────────────────────────┐
                    │   Aether Control Plane   │
                    │  deploy · migrate · spec │
                    └───────────┬─────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
   Podman/K8s              KubeVirt                 Metal3
        │                       │
        │              ┌────────┴────────┐
        │              │    Ragnarok     │  (optional composite)
        │              │ attestation ·   │
        │              │ measured images │
        │              └────────┬────────┘
        │                       │
        └───────────────────────┼───────────────────────┘
                                ▼
                    ┌─────────────────────────┐
                    │      PacketWolf         │
                    │   observe · telemetry   │
                    └───────────┬─────────────┘
                                ▼
                    ┌─────────────────────────┐
                    │       GuestKit          │
                    │    VM inspect · guest   │
                    └─────────────────────────┘

        HyperSDK (runtime SDK) ──► Aether + Zyra (control plane + AI OS)
```

---

## Product roles

| Product | Role |
|---------|------|
| **Aether** | Universal runtime portability — deploy, migrate, intent placement |
| **Zyra** | AI infrastructure operating layer built into Aether — multi-LLM, multi-agent, ambient intelligence (see [ZYRA.md](ZYRA.md)) |
| **Ragnarok** | Confidential execution layer — attestation, measured images, attest-gated secrets; standalone binary + UI, optional composite with Aether via `RAGNAROK_URL` |
| **PacketWolf** | Network/workload observability |
| **GuestKit** | Guest VM inspection and tooling |
| **HyperSDK** | Shared runtime SDK foundation |

---

## When to use Aether vs Kubernetes alone

Use Kubernetes alone when you have a single cluster and no VM/bare-metal path.

Use Aether when you need **one spec** across dev (Podman), prod (K8s), isolation (KubeVirt), and metal (Metal3) with **migration** and **explainable placement**.

---

## Links

- [Product overview](PRODUCT.md)
- [Product tiers](PRODUCT-TIERS.md)
- [Ragnarok and Aether — integration](guides/security/RAGNAROK-AND-AETHER.md)
- README — Zyvor platform section
