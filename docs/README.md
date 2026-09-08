# Aether Documentation

Universal runtime portability — deploy once, run anywhere

## Start Here

| Goal | Document |
|------|----------|
| Install | [01-Installation.md](getting-started/01-Installation.md) |
| Quick start | [02-Quick-Start.md](getting-started/02-Quick-Start.md) |
| Product overview | [PRODUCT.md](PRODUCT.md) |
| Ecosystem | [ECOSYSTEM.md](ECOSYSTEM.md) |
| Migration internals | [MIGRATION-INTERNALS.md](guides/migration/MIGRATION-INTERNALS.md) |
| Full index | [index.md](index.md) |
| **User journeys & acceptance criteria** | [User Stories](USER_STORIES.md) |

## User Stories

Persona-based journeys with acceptance criteria: **[USER_STORIES.md](USER_STORIES.md)**

| Persona | Focus |
|---------|-------|
| Alex (Platform Engineer) | Migrate workloads across Podman, K8s, KubeVirt |
| Morgan (DevOps Lead) | Blue-green and rolling migrations with zero downtime |
| Jordan (SRE) | Drift detection, health checks, and auto-reconciliation |
| Riley (Developer) | Single YAML spec for local Podman and prod K8s |

## Ecosystem

Part of the [Zyvor / HyperSDK platform stack](https://zyvor.dev):

| Product | Role |
|---------|------|
| **hypercluster** | Kubernetes bootstrap |
| **machina** | Bare-metal hypervisor OS |
| **zeus-os (v9s)** | Cloud / KubeVirt control plane |
| **forge** | AI infrastructure on K8s |
| **hypersdk / hyper2kvm** | VM migration |
| **guestkit** | Offline VM assurance |
| **packetwolf** | Network intelligence |
| **Aether** | Runtime portability |
| **hermes** | Application layer for K8s |

See also: [../README.md](../README.md)
