# 📘 Orchestr8 Documentation

> **One spec. Four runtimes. One tool.**
>
> Orchestr8 is a Universal Runtime Control Plane that deploys workloads to
> Podman, Kubernetes, KubeVirt, and Metal3 from a single YAML specification.

---

## 🚀 Quick Start Links

| Link | Description |
|------|-------------|
| [Installation Guide](getting-started/01-Installation.md) | Install orchestr8 from source, packages, or containers |
| [Quick Start (5 min)](getting-started/02-Quick-Start.md) | Deploy your first workload in under five minutes |
| [Complete Documentation Index](index.md) | Every page in the docs, organized by topic |

---

## 🆕 Latest Features (v0.3.0)

### 🎼 Compose Files
Deploy multi-workload stacks from a single `orchestr8-compose.yaml` with dependency ordering,
runtime overrides, and per-workload environment variables.

```bash
orchestr8 compose up
orchestr8 compose down
orchestr8 compose validate
```

### 🔌 Plugin System
Extend orchestr8 with third-party runtimes via a JSON-RPC plugin protocol. Plugins are
auto-discovered from `~/.orchestr8/plugins/`.

```bash
orchestr8 plugin discover
orchestr8 plugin list
orchestr8 plugin register my-runtime.json
```

### 💓 Health Tracking
Per-workload health history with uptime calculations, restart tracking, and historical timelines.
Background health-check loops with circuit-breaker protection.

```bash
orchestr8 health my-app --summary
orchestr8 orchestrate health-check
orchestr8 orchestrate watch --interval 30
```

### 🛡️ Policy Gate
Enforce production and development policies before deployment. Configurable policy sets
with dry-run support.

```bash
orchestr8 policy-check --policy production
orchestr8 run --skip-policy          # bypass (CI only)
```

### 🔐 AES-256 Secrets
Encrypted secrets management with rotation policies, access auditing, and namespace isolation.
Values are encrypted at rest using AES-256-GCM.

```bash
orchestr8 secrets create my-secret
orchestr8 secrets set my-secret DB_PASSWORD hunter2
orchestr8 secrets audit
```

### 🔄 Drift Detection & Reconciliation
Detect when running workloads diverge from their spec and auto-reconcile drifted state.

```bash
orchestr8 drift my-app
orchestr8 drift my-app --reconcile
```

### 🔁 Background Health Loop
Continuously monitor workloads with configurable intervals and circuit-breaker thresholds.

```bash
orchestr8 orchestrate watch --interval 30
orchestr8 orchestrate rolling-update my-app --replicas 3
orchestr8 orchestrate reset-circuit my-app
```

### 🖥️ TUI Dashboard
Interactive terminal UI with real-time workload status, log streaming, and keyboard navigation.

```bash
orchestr8 tui
```

---

## 📚 Documentation Hub

> **[Complete Documentation Index](index.md)** -- Every page in the docs,
> organized into Getting Started, Tutorials, Guides, Features, Deployment,
> Reference, and Quick Reference sections.

---

## 👥 Quick Access by Role

### 🔧 SysAdmin / Ops
- [Installation](getting-started/01-Installation.md) -- Get orchestr8 running on your infrastructure
- [Backup & Restore](BACKUP.md) -- State backup and disaster recovery procedures
- [Deployment Guide](DEPLOYMENT.md) -- Kubernetes, Helm, and container-based deployment
- [Runbook](RUNBOOK.md) -- Operational playbooks for common scenarios
- [Metrics & Monitoring](METRICS.md) -- Prometheus metrics export and dashboards

### 💻 Developer
- [Quick Start](getting-started/02-Quick-Start.md) -- From zero to deployed in five minutes
- [Workload Spec Schema](SCHEMA.md) -- Complete YAML specification reference
- [Templates](TEMPLATES.md) -- Generate workload specs from built-in templates
- [CI/CD Integration](CICD.md) -- GitHub Actions, GitLab CI, and Jenkins pipelines
- [Web Dashboard](WEBUI.md) -- REST API server and browser-based UI

### 🏢 Enterprise
- [Cost Estimation](COST.md) -- Multi-cloud cost analysis (AWS, Azure, GCP, DigitalOcean, Linode)
- [Migration Strategies](index.md#guides) -- Blue-green, rolling, and immediate migration
- [SLA Compliance](index.md#features) -- Uptime targets, latency budgets, error-rate monitoring
- [Audit Trail](index.md#features) -- Full event history with filtering and summaries

---

## 📋 Quick Access by Task

| Task | Command | Docs |
|------|---------|------|
| Deploy a workload | `orchestr8 run` | [Quick Start](getting-started/02-Quick-Start.md) |
| Deploy a directory of workloads | `orchestr8 deploy ./specs/` | [Quick Start](getting-started/02-Quick-Start.md) |
| Deploy a compose stack | `orchestr8 compose up` | [Index](index.md#features) |
| Migrate between runtimes | `orchestr8 migrate my-app kube` | [Index](index.md#guides) |
| Monitor workloads | `orchestr8 tui` | [Web UI](WEBUI.md) |
| Manage plugins | `orchestr8 plugin list` | [Index](index.md#features) |
| Check health history | `orchestr8 health my-app` | [Index](index.md#features) |
| Manage secrets | `orchestr8 secrets list` | [Index](index.md#features) |
| Detect drift | `orchestr8 drift my-app` | [Index](index.md#features) |
| Estimate costs | `orchestr8 cost` | [Cost Estimation](COST.md) |
| Generate shell completions | `orchestr8 completions bash` | [Installation](getting-started/01-Installation.md) |
| Start API server | `orchestr8 serve` | [Web UI](WEBUI.md) |

---

## 🗂️ Documentation Structure

```
docs/
├── README.md                          # This file -- documentation hub
├── index.md                           # Complete documentation index
├── getting-started/
│   ├── 01-Installation.md             # Prerequisites, build, packages, verify
│   └── 02-Quick-Start.md              # 5-minute quick start guide
├── SCHEMA.md                          # Workload YAML specification reference
├── TEMPLATES.md                       # Built-in workload templates
├── COST.md                            # Multi-cloud cost estimation
├── METRICS.md                         # Prometheus metrics export
├── DEPLOYMENT.md                      # Kubernetes/Helm/container deployment
├── CICD.md                            # CI/CD pipeline integration
├── BACKUP.md                          # Backup and restore procedures
├── WEBUI.md                           # REST API and web dashboard
└── RUNBOOK.md                         # Operational playbooks
```

---

## 🏗️ Supported Runtimes

| Runtime | CLI Alias | Use Case |
|---------|-----------|----------|
| **Podman** | `podman`, `container` | Local dev, rootless containers, single-node |
| **Kubernetes** | `kube`, `k8s`, `kubernetes` | Production clusters, horizontal scaling |
| **KubeVirt** | `kubevirt`, `vm` | VM workloads on Kubernetes, legacy apps |
| **Metal3** | `metal3`, `metal`, `bare-metal` | Bare-metal provisioning, high performance |

---

## 📖 Migration Strategies

| Strategy | Downtime | Safety | Use Case |
|----------|----------|--------|----------|
| **Immediate** | Brief | Rollback on failure | Dev/staging, fast iteration |
| **Blue-Green** | Zero | Target validated before switch | Production, critical services |
| **Rolling** | Zero | Gradual traffic shift with health checks | Large-scale, canary-style |

---

## 📄 License

Proprietary -- HyperSDK. See [LICENSE](../LICENSE) for details.

---

*Generated for orchestr8 v0.3.0. See the [Complete Index](index.md) for all documentation.*
