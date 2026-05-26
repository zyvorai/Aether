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
        │                       │                       │
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

        HyperSDK (runtime SDK) ──► Zeus (orchestration layer)
```

---

## Product roles

| Product | Role |
|---------|------|
| **Aether** | Universal runtime portability — deploy, migrate, intent placement |
| **PacketWolf** | Network/workload observability |
| **GuestKit** | Guest VM inspection and tooling |
| **HyperSDK** | Shared runtime SDK foundation |
| **Zeus** | Higher-level orchestration (platform) |

---

## When to use Aether vs Kubernetes alone

Use Kubernetes alone when you have a single cluster and no VM/bare-metal path.

Use Aether when you need **one spec** across dev (Podman), prod (K8s), isolation (KubeVirt), and metal (Metal3) with **migration** and **explainable placement**.

---

## Links

- [Product overview](PRODUCT.md)
- [Product tiers](PRODUCT-TIERS.md)
- README — Zyvor platform section
