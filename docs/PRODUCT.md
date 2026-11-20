---
hero:
  eyebrow: "PRODUCT"
  title: "Universal Runtime Portability"
  lead: "Deploy once. Move workloads across containers, Kubernetes, and VMs without rewriting infrastructure."
  tone: violet
  swatches:
    - {label: "Podman — local dev & edge", tone: sky}
    - {label: "Kubernetes — cluster orchestration", tone: violet}
    - {label: "KubeVirt — VM isolation, GPU", tone: teal}
  highlights:
    - {value: "3", label: "Runtimes, one spec"}
    - {value: "12", label: "Runtime migration pairs", footnote: "1"}
    - {value: "4", label: "Migration strategies", footnote: "2"}
footnotes:
  - {marker: "1", text: "4 runtime kinds (Podman, Docker, Kubernetes, KubeVirt) × 3 valid targets each, source ≠ target — enforced in the migration engine's guard clause.", href: "guides/migration/MIGRATION-INTERNALS.md#limits-matrix", href_label: "See the limits matrix."}
  - {marker: "2", text: "Immediate, Blue-Green, Rolling, and Canary.", href: "guides/migration/MIGRATION-INTERNALS.md", href_label: "See Migration Internals."}
---

## The problem

Teams run the same application on Podman locally, Kubernetes in production, and KubeVirt for GPU/VM isolation — but each runtime has its own toolchain, config language, and operational model. Moving between them means rewriting manifests, re-learning networking, and accepting migration risk.

## The differentiator

**Universal runtime portability.** One Aether workload spec (`aether/v1`) describes what to run. Aether deploys it to the right runtime, explains why, and migrates it between runtimes with production strategies (immediate, blue-green, rolling, canary).

| Runtime | Role |
|---------|------|
| Podman | Local dev and edge containers |
| Kubernetes | Cluster orchestration, services, scaling |
| KubeVirt | VM isolation, GPU passthrough |

## Three proof points

1. **One spec, three runtimes** — Validate once, deploy anywhere. See [Schema Reference](reference/SCHEMA.md) and [Quick Start](getting-started/02-Quick-Start.md).

2. **Production migration** — 12 runtime pairs, rollback on failure, health gates, connection draining. See [Migration Guide](guides/migration/MIGRATION-GUIDE.md) and [Migration Internals](guides/migration/MIGRATION-INTERNALS.md).

3. **Explainable placement** — Intent-driven scoring ranks runtimes with reasons, not black-box picks. Run `aether decide --spec workload.yaml --explain` or use the dashboard AI Engine.

## Compare runtimes

<div class="compare-cards" markdown="1">
- **Podman**<br>Local dev and edge containers. No cluster required; fastest path from spec to running workload.
- **Kubernetes**<br>Cluster orchestration, services, scaling. The production default for multi-node, horizontally-scaled workloads.
- **KubeVirt**<br>VM isolation and GPU passthrough on top of Kubernetes, for legacy VM workloads and hardware-bound GPU jobs.
</div>

## Security

<div class="icon-badge-list" markdown="1">
- 🔒 AES-256-GCM encryption
- 🔑 RBAC API integration
- 📜 Audit trail integrity
- 🛡️ Policy engine
- ⏱️ Rate limiting
- 🔗 Webhook delivery security
</div>

[Full security reference →](features/security.md)

## Zyra — the AI layer

Zyra is Aether's ambient AI layer, not a bolt-on chatbot — present throughout the CLI and dashboard.

- **Multi-LLM** — OpenAI, Anthropic, Gemini, xAI Grok, Azure, Ollama, vLLM, and OpenAI-compatible endpoints
- **Multi-agent** — Auto, Architect, DevOps, Kubernetes, Security, SRE, Cost, Observability, AI Engineer, Database Expert
- **Intelligent routing** — Task-class based provider and agent selection
- **Approval-gated actions** — Mutations require explicit user confirmation

[Full Zyra reference →](reference/ZYRA.md)

## Integrations

=== "Atlas (storage)"

    [Atlas](https://github.com/ssahani/atlas) is the Zyvor storage control plane. When enabled, Aether provisions persistent storage for a workload through Atlas instead of creating a native Kubernetes PVC itself.

    [Atlas integration guide →](integrations/ATLAS.md)

=== "Cloud Matrix"

    Vendor support levels for Aether's target runtimes across AWS EKS, Google GKE, and other managed Kubernetes offerings.

    [Cloud Integration Matrix →](integrations/CLOUD-MATRIX.md)

=== "Forge (GPU / AI)"

    [Forge](https://github.com/ssahani/forge) is the Zyvor AI infrastructure control plane. Aether reads GPU capacity, node inventory, placement recommendations, and cost from the Forge API gateway — read-only.

    [Forge integration guide →](integrations/FORGE.md)

## Who it's for

- Platform teams standardizing dev → prod paths
- SREs moving workloads off legacy VM or container stacks
- Architects evaluating hybrid cloud + bare metal + K8s from one control plane

## Get started

```bash
aether init
aether validate --spec workload.yaml
aether run --spec workload.yaml
aether decide --spec workload.yaml --explain
aether migrate my-app kubernetes --strategy blue-green
```

## Learn more

- [Documentation index](index.md)
- [Architecture](architecture/ARCHITECTURE.md)
- [Roadmap](ROADMAP.md)
- [Ecosystem (Zyvor platform)](ECOSYSTEM.md)

## Contact

Enterprise onboarding: see customer bundle `START_HERE.txt` and [Production Reference](deployment/PRODUCTION-REFERENCE.md).
