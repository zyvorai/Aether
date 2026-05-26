# Aether — Universal Runtime Portability

> **Deploy once. Move workloads across containers, Kubernetes, VMs, and bare metal without rewriting infrastructure.**

## The problem

Teams run the same application on Podman locally, Kubernetes in production, KubeVirt for GPU/VM isolation, and Metal3 for bare metal — but each runtime has its own toolchain, config language, and operational model. Moving between them means rewriting manifests, re-learning networking, and accepting migration risk.

## The differentiator

**Universal runtime portability.** One Aether workload spec (`aether/v1`) describes what to run. Aether deploys it to the right runtime, explains why, and migrates it between runtimes with production strategies (immediate, blue-green, rolling, canary).

| Runtime | Role |
|---------|------|
| Podman | Local dev and edge containers |
| Kubernetes | Cluster orchestration, services, scaling |
| KubeVirt | VM isolation, GPU passthrough |
| Metal3 | Bare-metal performance |

## Three proof points

1. **One spec, four runtimes** — Validate once, deploy anywhere. See [Schema Reference](SCHEMA.md) and [Quick Start](getting-started/02-Quick-Start.md).

2. **Production migration** — 16 runtime pairs, rollback on failure, health gates, connection draining. See [Migration Guide](guides/migration/MIGRATION-GUIDE.md) and [Migration Internals](guides/migration/MIGRATION-INTERNALS.md).

3. **Explainable placement** — Intent-driven scoring ranks runtimes with reasons, not black-box picks. Run `aether decide --spec workload.yaml --explain` or use the dashboard AI Engine.

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
- [Product tiers](PRODUCT-TIERS.md)
- [Roadmap](ROADMAP.md)
- [Ecosystem (Zyvor platform)](ECOSYSTEM.md)

## Contact

Enterprise onboarding: see customer bundle `START_HERE.txt` and [Production Reference](deployment/PRODUCTION-REFERENCE.md).
