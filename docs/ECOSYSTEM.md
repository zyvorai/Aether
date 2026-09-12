---
hero:
  eyebrow: "ECOSYSTEM"
  title: "Where Aether fits in the Zyvor suite."
  lead: "Aether is the control plane. Zyra, Ragnarok, PacketWolf, and GuestKit plug in around it."
  tone: teal
  highlights:
    - {value: "5", label: "Suite products"}
    - {value: "1", label: "Control plane"}
---

## Role diagram

```
                    ┌─────────────────────────┐
                    │   Aether Control Plane   │
                    │  deploy · migrate · spec │
                    └───────────┬─────────────┘
                                │
                    ┌───────────┴───────────┐
                    ▼                       ▼
               Podman/K8s               KubeVirt
                    │                       │
                    │              ┌────────┴────────┐
                    │              │    Ragnarok     │  (optional composite)
                    │              │ attestation ·   │
                    │              │ measured images │
                    │              └────────┬────────┘
                    │                       │
                    └───────────┬───────────┘
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
| **Zyra** | AI infrastructure operating layer built into Aether — multi-LLM, multi-agent, ambient intelligence (see [ZYRA.md](reference/ZYRA.md)) |
| **Ragnarok** | Confidential execution layer — attestation, measured images, attest-gated secrets; standalone binary + UI, optional composite with Aether via `RAGNAROK_URL` |
| **PacketWolf** | Network/workload observability |
| **GuestKit** | Guest VM inspection and tooling |
| **HyperSDK** | Shared runtime SDK foundation |

---

## When to use Aether vs Kubernetes alone

Use Kubernetes alone when you have a single cluster and no VM/bare-metal path.

Use Aether when you need **one spec** across dev (Podman), prod (K8s), and isolation (KubeVirt) with **migration** and **explainable placement**.

---

## Links

- [Product overview](PRODUCT.md)
- [README](https://github.com/zyvorai/Aether/blob/main/README.md) — Zyvor stack note
- [zyvor.dev](https://zyvor.dev) — Confidential computing (**Ragnarok**) is a separate product
