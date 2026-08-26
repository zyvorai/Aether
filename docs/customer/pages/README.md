# Page-by-page guides

Each guide follows: Purpose → When to use it → How to get there → What you can do → Related pages.

Every route is also listed in the [complete page index](../PAGE_INDEX.md).

## Intelligence

| Page | What it covers |
|------|----------------|
| [Runtime Affinity](intelligence/affinity.md) | Workload class affinity and runtime fit |
| [AI Engine](intelligence/ai.md) | Intent scoring, runtime recommendations & migration planning |
| [Confidential Computing](intelligence/confidential.md) | TEE capabilities, attestation trust scores, and Ragnarok integration |
| [Cost Estimation](intelligence/cost.md) | Resource cost projections across runtimes |
| [Drift Detection](intelligence/drift.md) | Configuration drift & desired-state reconciliation |
| [Intelligence Layer](intelligence/intelligence.md) | Failure predictions, threats, cost optimization, and global placement |
| [Migrations](intelligence/migrations.md) | AI migration planner with risk analysis and strategy |
| [Policy Check](intelligence/policy.md) | Validate workloads against policy rules |
| [Security Center](intelligence/security.md) | Threats, secrets, policies, and hardening |
| [Zyra](intelligence/zyra.md) | AI infrastructure operating layer — multi-LLM, multi-agent intelligence |

## Operations

| Page | What it covers |
|------|----------------|
| [Activity Monitor](operations/activity.md) | CPU, memory, restarts, and errors across the fleet |
| [Alerts](operations/alerts.md) | Notification channels, alert rules, and webhook tests |
| [Cluster Browser](operations/clusters.md) | Browse and manage Kubernetes resources |
| [Compose Import](operations/compose.md) | Import Docker Compose into Aether workloads |
| [Dependencies](operations/deps.md) | Workload dependency graph |
| [Visual Editor](operations/editor.md) | Form-based workload designer (no YAML required) |
| [Environments](operations/envs.md) | Environment tiers and configuration |
| [Events](operations/events.md) | Platform and workload events |
| [Fleet Overview](operations/fleet.md) | Multi-cluster inventory, Hubble, and PacketWolf links |
| [Health Monitor](operations/health-monitor.md) | Workload health checks and status |
| [Helm App Store](operations/helm.md) | Install curated charts with a guided wizard |
| [Observability](operations/observability.md) | Health, metrics, events, and correlated diagnostics |
| [Platform](operations/platform.md) | HA mode, setup recommendations, OPA, and observability |
| [Scheduler](operations/scheduler.md) | Scheduling recommendations and placement |
| [SLA Compliance](operations/sla.md) | SLA tracking and compliance |

## Primary

| Page | What it covers |
|------|----------------|
| [Applications](primary/applications.md) | Manage Kubernetes apps like an operating system — not like YAML |
| [Runtime Fabric](primary/fabric.md) | Live Application → Runtime → Cluster → Node topology |
| [Overview](primary/home.md) | Fleet briefing, savings, and capacity intelligence |
| [Workloads](primary/workloads.md) | Deploy, monitor, and manage across Podman, Kubernetes, KubeVirt & Metal3 |

## Resources

| Page | What it covers |
|------|----------------|
| [Audit Trail](resources/audit.md) | Tamper-evident audit log |
| [Backups](resources/backups.md) | Backup snapshots and restore |
| [GPU / Forge](resources/forge.md) | Forge GPU capacity, nodes, and AI placement |
| [GitOps](resources/gitops.md) | GitOps reconciliation status |
| [Hosted SaaS](resources/hosted.md) | Tenants, API keys, metering, and Stripe billing |
| [Labs](resources/labs.md) | Experimental AI-generated infrastructure artifacts |
| [Metrics](resources/metrics.md) | Platform and workload metrics |
| [API Explorer](resources/openapi.md) | Browse OpenAPI routes and raw schema |
| [Plugins](resources/plugins.md) | Runtime plugins and extensions |
| [Access Control](resources/rbac.md) | API keys and role-based access |
| [Secrets](resources/secrets.md) | Encrypted secrets management |
| [AI Providers](resources/settings-ai-providers.md) | Configure OpenAI, Claude, Gemini, Grok, Ollama, and custom LLM endpoints |
| [Settings](resources/settings.md) | Platform, environments, secrets, and extensions |
| [Storage](resources/storage.md) | Atlas-backed persistent volumes and capacity |
| [Templates](resources/templates.md) | Workload templates library |

---

44 guides. Regenerate: `node scripts/customer-docs/generate-guide-index.mjs`.
