---
hero:
  eyebrow: "AETHER"
  title: "Universal runtime portability."
  lead: "Deploy once. Move workloads across Podman, Kubernetes, and KubeVirt without rewriting infrastructure."
  tone: sky
  swatches:
    - {label: "v0.4.0", tone: sky}
    - {label: "Apache-2.0", tone: emerald}
  highlights:
    - {value: "3", label: "Runtimes unified"}
    - {value: "16", label: "Migration pairs"}
    - {value: "40+", label: "CLI commands"}
    - {value: "40+", label: "REST API endpoints"}
  hub_bands:
    - {icon: "◆", title: "Product overview", description: "Why Aether exists and where it fits.", href: "PRODUCT.md", tone: sky}
    - {icon: "⇄", title: "Migration credibility", description: "State machine, rollback, and the limits matrix.", href: "guides/migration/MIGRATION-INTERNALS.md", tone: violet}
    - {icon: "◎", title: "Runtime decisions", description: "Scoring engine weights, intent, and explain output.", href: "guides/decision-engine/SCORING.md", tone: amber}
    - {icon: "▣", title: "Enterprise trust", description: "Deployment topologies and the production reference.", href: "architecture/DEPLOYMENT-TOPOLOGIES.md", tone: emerald}
    - {icon: "◈", title: "Ecosystem", description: "Where Aether fits in the Zyvor suite.", href: "ECOSYSTEM.md", tone: teal}
---

## Trust and proof

| Document | Description |
|----------|-------------|
| [Migration Internals](guides/migration/MIGRATION-INTERNALS.md) | State machine, rollback, limits matrix |
| [Stateful Portability](guides/migration/STATEFUL-PORTABILITY.md) | Volumes, databases, honest boundaries |
| [Scoring / Decision Engine](guides/decision-engine/SCORING.md) | Weights, intent, explain output |
| [Deployment Topologies](architecture/DEPLOYMENT-TOPOLOGIES.md) | Single-node, HA, hybrid, edge |
| [Fleet Architecture](architecture/FLEET-ARCHITECTURE.md) | Multi-cluster now vs roadmap |
| [Networking](architecture/NETWORKING.md) | Runtime translation, DNS, ingress |
| [Production Reference](deployment/PRODUCTION-REFERENCE.md) | Scale, recovery, upgrades |
| [Benchmarks](https://github.com/zyvorai/Aether/blob/main/benchmarks/RESULTS.md) | Deploy/migrate/API baselines |
| [Cloud Matrix](integrations/CLOUD-MATRIX.md) | Vendor support levels |
| [Atlas Storage](integrations/ATLAS.md) | Atlas-backed persistent volumes |
| [Forge GPU / AI](integrations/FORGE.md) | GPU capacity, nodes, placement, cost |
| [Ecosystem](ECOSYSTEM.md) | Aether + Zyvor suite |
| [Roadmap](ROADMAP.md) | Ship vs planned |

---

## Table of Contents

1. [Getting Started](#-getting-started)
2. [Tutorials](#-tutorials)
3. [Guides](#-guides)
4. [Features](#-features)
5. [Deployment](#-deployment)
6. [Reference](#-reference)
7. [Quick Reference](#-quick-reference)
8. [Learning Paths](#-learning-paths)

---

## 🚀 Getting Started

| Document | Description | Time |
|----------|-------------|------|
| [Installation Guide](getting-started/01-Installation.md) | Prerequisites, build from source, package managers, container image, Helm chart, shell completions, first-time setup, verification | 5 min |
| [Quick Start (5 min)](getting-started/02-Quick-Start.md) | Create, validate, build, deploy, status, logs, migrate, TUI -- all in five minutes | 10 min |

---

## 🎓 Tutorials

### Beginner

| Topic | Document | What You Learn | Time |
|-------|----------|----------------|------|
| First Deployment | [Beginner Tutorial](tutorials/01-beginner-deployment.md) | Create, deploy, monitor, and delete a workload | 30 min |
| Templates | [Templates Guide](reference/TEMPLATES.md) | Generate workload specs from 8 built-in templates (`web-app`, `rest-api`, `database`, `cache`, `worker`, `cron-job`, `ml-training`, `microservice`) | 15 min |
| Shell Completions | [Installation Guide](getting-started/01-Installation.md#-shell-completions) | Tab completion for bash, zsh, fish, PowerShell, elvish | 5 min |

### Intermediate

| Topic | Document | What You Learn | Time |
|-------|----------|----------------|------|
| Workflows | [Intermediate Workflows](tutorials/02-intermediate-workflows.md) | Compose files, migrations, output formats, watch mode | 45 min |
| Workload Schema | [Schema Reference](reference/SCHEMA.md) | Full YAML specification: metadata, build, requirements, runtime, network, persistence, health, config, ingress, scaling | 20 min |
| Cost Analysis | [Cost Estimation](guides/operations/COST.md) | Compare costs across AWS, Azure, GCP, DigitalOcean, Linode | 15 min |

### Advanced

| Topic | Document | What You Learn | Time |
|-------|----------|----------------|------|
| Advanced Features | [Advanced Tutorial](tutorials/03-advanced-features.md) | Policies, AES-256 secrets, drift reconciliation, plugins | 60 min |
| How It Works | [Deep Dive Tutorial](tutorials/04-how-it-works.md) | Internal architecture, decision engine, runtime adapters, state management, security model | 90 min |
| CI/CD Integration | [CI/CD Guide](guides/operations/CICD.md) | GitHub Actions, GitLab CI, Jenkins pipelines | 30 min |

---

## 📗 Guides

### Architecture

| Guide | Description |
|-------|-------------|
| [Architecture Overview](architecture/ARCHITECTURE.md) | System design, component architecture, data flow, security model, key design decisions |

### CLI Guide

| Guide | Description |
|-------|-------------|
| [CLI Reference](guides/cli/CLI-Reference.md) | All 40+ commands with flags, descriptions, and examples |
| [Quick Reference Card](quick-reference/QUICK_REFERENCE.md) | One-page command cheat sheet |

#### Core Lifecycle Commands

| Command | Description |
|---------|-------------|
| `aether init` | First-time setup wizard |
| `aether validate` | Validate workload YAML specification |
| `aether build` | Build workload container image |
| `aether run [--runtime <rt>]` | Deploy a workload (optional runtime override) |
| `aether stop <name>` | Stop a running workload |
| `aether status <name>` | Get workload instance status |
| `aether logs <name> [--follow]` | View workload logs |
| `aether delete <name>` | Delete a workload instance |
| `aether list` | List all deployed workloads |
| `aether exec <name> [cmd]` | Execute a command inside a running workload |
| `aether port-forward <name> <local:remote>` | Forward local ports to a workload |
| `aether watch [--runtime <rt>]` | Watch spec file and auto-redeploy on changes |

#### Batch Operations

| Command | Description |
|---------|-------------|
| `aether deploy <dir> [--runtime <rt>] [--fail-fast] [--dry-run]` | Deploy all workloads from a directory |
| `aether compose up [file] [--runtime <rt>] [--dry-run]` | Deploy all workloads from a compose file |
| `aether compose down [file]` | Stop all workloads from a compose file |
| `aether compose validate [file]` | Validate a compose file |

#### Migration

| Command | Description |
|---------|-------------|
| `aether migrate <name> <target> [--strategy <s>]` | Migrate workload to a different runtime |
| `aether migration-advice <name> <target>` | AI-powered migration path recommendations |
| `aether rollback <name>` | Rollback to the latest snapshot |
| `aether diff <name>` | Compare spec vs stored vs live state |

#### AI & Analysis

| Command | Description |
|---------|-------------|
| `aether recommend` | AI-powered runtime recommendation with scoring |
| `aether profile [--name <n>]` | Workload profiling and optimization recommendations |
| `aether analyze-logs <name>` | Anomaly and pattern detection in logs |
| `aether scaling-advice` | Predictive scaling recommendations |
| `aether compare` | Compare workload across runtimes (cost, capabilities) |
| `aether affinity recommend <class>` | Runtime affinity for a workload class |
| `aether affinity matrix` | Full compatibility matrix |

#### Infrastructure & Scheduling

| Command | Description |
|---------|-------------|
| `aether cost [--provider <p>]` | Multi-cloud cost estimation |
| `aether config [--show] [--init]` | Show or initialize configuration |
| `aether template <name> [--output <file>]` | Generate workload from template |
| `aether env create <name> [--tier <t>]` | Create deployment environment |
| `aether env promote <workload> <from> <to>` | Promote workload between environments |
| `aether schedule place <name> [--strategy <s>]` | Schedule workload placement |
| `aether schedule utilization` | Show runtime utilization |

#### Observability

| Command | Description |
|---------|-------------|
| `aether tui` | Interactive TUI dashboard |
| `aether serve [--host <h>] [--port <p>]` | Start API server and web dashboard |
| `aether metrics` | Export Prometheus metrics |
| `aether health <name> [--summary]` | View workload health history and uptime |
| `aether events [--last <n>] [--severity <s>]` | View and filter events |
| `aether audit [--last <n>] [--workload <w>]` | View audit trail |
| `aether sla check <workload> --uptime <u>` | Check SLA compliance |

#### Security & Compliance

| Command | Description |
|---------|-------------|
| `aether secrets create <name>` | Create a new encrypted secret |
| `aether secrets set <secret> <key> <value>` | Set a key-value pair (AES-256 encrypted) |
| `aether secrets get <secret> <key>` | Retrieve a decrypted value |
| `aether secrets list` | List all secrets |
| `aether secrets audit` | Check rotation status |
| `aether policy-check [--policy <p>]` | Check workload against policies |
| `aether drift <name> [--reconcile]` | Detect and reconcile configuration drift |

#### Operations

| Command | Description |
|---------|-------------|
| `aether backup [--name <n>]` | Backup workload state |
| `aether restore <backup> [--merge]` | Restore state from backup |
| `aether list-backups` | List available backups |
| `aether deps show` | Show dependency graph |
| `aether deps impact <workload>` | Show impact of stopping a workload |
| `aether webhook add <name> <url>` | Add webhook notification channel |
| `aether webhook test <name>` | Send test notification |

#### Health-Aware Orchestration

| Command | Description |
|---------|-------------|
| `aether orchestrate register <name>` | Register workload for health monitoring |
| `aether orchestrate status` | Health status of all managed workloads |
| `aether orchestrate health-check` | Run a single round of health checks |
| `aether orchestrate watch [--interval <s>]` | Continuous health monitoring loop |
| `aether orchestrate rolling-update <name>` | Rolling update with configurable replicas |
| `aether orchestrate reset-circuit <name>` | Reset circuit breaker |

### Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--spec <file>` | `-s` | Workload spec file (default: `workload.yaml`) |
| `--verbose` | `-v` | Enable debug logging |
| `--quiet` | `-q` | Suppress all output except errors |
| `--json` | | Output as JSON |
| `--output <fmt>` | `-o` | Output format: `table`, `json`, `yaml`, `wide` |
| `--yes` | `-y` | Skip confirmation prompts (CI/automation) |
| `--dry-run` | | Show what would happen without executing |
| `--skip-policy` | | Skip policy checks on deploy |

### Operations Guide

| Guide | Description |
|-------|-------------|
| [Migration Checklist](guides/operations/MIGRATION_CHECKLIST.md) | Pre/post migration steps, strategy selection, rollback procedures |
| [Operational Runbook](guides/operations/RUNBOOK.md) | Troubleshooting playbooks and recovery procedures |
| [Backup & Restore](guides/operations/BACKUP.md) | State backup, disaster recovery, merge restore |

---

## ⭐ Features

| Feature | Document | Description |
|---------|----------|-------------|
| Compose Files | [Compose Guide](features/compose.md) | Multi-workload deployment with dependency ordering, runtime overrides, env injection |
| Plugin System | [Plugins Guide](features/plugins.md) | Runtime extension via JSON manifest discovery and JSON-RPC protocol |
| Health Monitoring | [Health Guide](features/health-monitoring.md) | Uptime tracking, restart history, timeline views, bounded storage |
| Security | [Security Guide](features/security.md) | AES-256 encryption, policy engine, state locking |
| Drift Detection | -- | Field-level drift reports with severity classification and auto-reconciliation |
| Migration Strategies | -- | Immediate, blue-green, and rolling cross-runtime migration with rollback |
| Runtime Affinity | -- | AI-powered runtime recommendations per workload class (8 classes) |
| SLA Compliance | -- | Uptime/latency/error-rate targets per workload (3 tiers) |
| Dependency Management | -- | Directed dependency graph, topological ordering, impact analysis |
| Webhook Notifications | -- | Push-based alerting with severity filtering and retry queues |
| Environment Management | -- | dev/staging/production tiers, promote, parity checking |
| Workload Scheduling | -- | 4 strategies: balanced, cost, performance, bin-packing |
| Cost Estimation | [Cost Guide](guides/operations/COST.md) | Multi-cloud analysis across AWS, Azure, GCP, DigitalOcean, Linode |
| Templates | [Templates Guide](reference/TEMPLATES.md) | 8 built-in templates: web-app, rest-api, database, cache, worker, cron-job, ml-training, microservice |

---

## 🚢 Deployment

| Document | Description |
|----------|-------------|
| [Deployment Guide](deployment/DEPLOYMENT.md) | Kubernetes manifests, Helm chart, container image deployment |
| [CI/CD Integration](guides/operations/CICD.md) | GitHub Actions, GitLab CI, Jenkins pipeline recipes |
| [Backup & Restore](guides/operations/BACKUP.md) | State backup, disaster recovery, merge restore |

---

## 📐 Reference

| Document | Description |
|----------|-------------|
| [Workload Spec Schema](reference/SCHEMA.md) | Complete YAML specification reference with all fields |
| [REST API Reference](reference/api/API-Reference.md) | 40+ REST API endpoints, request/response schemas |
| [Prometheus Metrics](guides/operations/METRICS.md) | All exported metrics, labels, and types |
| [Cost Models](guides/operations/COST.md) | Provider-specific cost models and formulas |
| [Templates Catalog](reference/TEMPLATES.md) | Built-in template catalog and customization |
| [Operational Runbook](guides/operations/RUNBOOK.md) | Troubleshooting playbooks and recovery procedures |
| [Web Dashboard](reference/WEBUI.md) | REST API server and browser-based UI |

### REST API Endpoints (Summary)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/api/workloads` | List all workloads |
| `POST` | `/api/workloads` | Create and deploy a workload |
| `GET` | `/api/workloads/:name` | Get workload details |
| `DELETE` | `/api/workloads/:name` | Delete a workload |
| `GET` | `/api/workloads/:name/logs` | Get workload logs |
| `POST` | `/api/workloads/:name/start` | Start a workload |
| `POST` | `/api/workloads/:name/stop` | Stop a workload |
| `POST` | `/api/workloads/:name/migrate` | Migrate to a different runtime |
| `POST` | `/api/workloads/:name/build` | Trigger a build |
| `POST` | `/api/validate` | Validate a workload YAML |
| `POST` | `/api/cost` | Estimate costs |
| `GET` | `/api/metrics` | Prometheus metrics (text/plain) |
| `GET` | `/api/backups` | List backups |
| `POST` | `/api/backups` | Create a backup |
| `GET` | `/api/secrets` | List secrets |
| `GET` | `/api/secrets/:name` | Get secret metadata |
| `DELETE` | `/api/secrets/:name` | Delete a secret |
| `POST` | `/api/ai/recommend` | AI runtime recommendation |
| `GET` | `/api/ai/profile/:name` | Workload profiling |
| `GET` | `/api/ai/analyze/:name` | Log anomaly analysis |
| `GET` | `/api/ai/migration-advice/:name/:target` | Migration advice |
| `GET` | `/api/ai/scaling-advice` | Predictive scaling |
| `GET` | `/api/drift/:name` | Drift detection |
| `POST` | `/api/policy/check` | Policy evaluation |
| `GET` | `/api/dependencies` | Dependency graph |
| `POST` | `/api/dependencies` | Add a dependency |
| `GET` | `/api/audit` | Audit events |
| `GET` | `/api/templates` | List templates |
| `POST` | `/api/templates/:name` | Generate from template |
| `GET` | `/api/sla/:workload` | SLA compliance |
| `GET` | `/api/events` | Recent events |
| `GET` | `/api/events/summary` | Event summary |
| `GET` | `/api/environments` | List environments |
| `GET` | `/api/scheduler/utilization` | Runtime utilization |
| `GET` | `/api/scheduler/optimize` | Optimization suggestions |
| `GET` | `/api/orchestrator/status` | Managed workload statuses |
| `GET` | `/api/orchestrator/summary` | Health summary |
| `GET` | `/api/affinity/:class` | Runtime affinity recommendation |
| `GET` | `/api/plugins` | List plugins |
| `POST` | `/api/plugins/discover` | Discover plugins |
| `GET` | `/api/health/:workload` | Health history summary |
| `POST` | `/api/compose/validate` | Validate compose spec |

---

## 🃏 Quick Reference

| Item | Document | Description |
|------|----------|-------------|
| Cheat Sheet | [Quick Reference Card](quick-reference/QUICK_REFERENCE.md) | One-page command cheat sheet |
| Hub | [Documentation Hub](https://github.com/zyvorai/Aether/blob/main/docs/README.md) | Landing page with role-based quick access |

### Glossary

| Term | Definition |
|------|-----------|
| **Workload** | A deployable unit described by a YAML spec (`workload.yaml`) |
| **Runtime** | A deployment target: Podman, Kubernetes, or KubeVirt |
| **RuntimeKind** | Enum identifying one of the three supported runtimes |
| **Instance** | A running workload on a specific runtime |
| **Migration** | Moving a workload from one runtime to another |
| **Drift** | Divergence between declared spec and actual live state |
| **Compose** | A multi-workload deployment defined in `aether-compose.yaml` |
| **Plugin** | A third-party runtime extension registered via JSON manifest |
| **Policy** | A set of rules evaluated before deployment is allowed |
| **SLA Target** | Uptime/latency/error-rate thresholds for a workload |
| **Circuit Breaker** | Automatic protection that halts health checks after repeated failures |
| **Blue-Green** | Migration strategy running both versions simultaneously before switching |
| **Rolling** | Migration strategy shifting traffic gradually (25%/50%/75%/100%) |
| **Affinity** | AI-learned runtime preference for a workload class |

### FAQ

**Q: Which runtime should I use?**
A: Run `aether recommend` for AI-powered scoring, or set `runtime.preferred: auto` in your spec for automatic selection.

**Q: Can I migrate between any two runtimes?**
A: Yes. All 12 runtime-pair combinations are supported (4 source x 3 target).

**Q: Is the spec format compatible with Kubernetes YAML?**
A: The aether spec is its own format (`apiVersion: aether/v1`). Use `aether template` to generate specs from common patterns.

**Q: How do I run aether in CI/CD?**
A: Use `--yes --quiet --json` flags for non-interactive, machine-readable output. See the [CI/CD Guide](guides/operations/CICD.md).

**Q: Where is state stored?**
A: In `~/.aether/` by default. Use `aether backup` and `aether restore` for portability.

---

## 🗺️ Learning Paths

### Beginner Path

```
Installation  -->  Quick Start  -->  First Deployment  -->  Quick Reference
```

1. [Install aether](getting-started/01-Installation.md)
2. [Deploy your first workload](getting-started/02-Quick-Start.md)
3. [First Deployment Tutorial](tutorials/01-beginner-deployment.md)
4. [Quick Reference Card](quick-reference/QUICK_REFERENCE.md)

### Intermediate Path

```
Workflows  -->  Compose  -->  Health  -->  CLI Reference  -->  Cost
```

1. [Intermediate Workflows](tutorials/02-intermediate-workflows.md)
2. [Compose Guide](features/compose.md)
3. [Health Monitoring](features/health-monitoring.md)
4. [CLI Reference](guides/cli/CLI-Reference.md)
5. [Cost Estimation](guides/operations/COST.md)

### Advanced Path

```
Advanced Features  -->  Security  -->  Plugins  -->  REST API  -->  CI/CD
```

1. [Advanced Features Tutorial](tutorials/03-advanced-features.md)
2. [Security Guide](features/security.md)
3. [Plugin System](features/plugins.md)
4. [REST API Reference](reference/api/API-Reference.md)
5. [CI/CD Integration](guides/operations/CICD.md)

### Enterprise Path

```
Migration Checklist  -->  Security  -->  Deployment  -->  REST API  -->  Runbook
```

1. [Migration Checklist](guides/operations/MIGRATION_CHECKLIST.md)
2. [Security Guide](features/security.md)
3. [Deployment Guide](deployment/DEPLOYMENT.md)
4. [REST API Reference](reference/api/API-Reference.md)
5. [Operational Runbook](guides/operations/RUNBOOK.md)

---

## 📄 License

Proprietary - Copyright (c) 2024-2026 HyperSDK. All rights reserved.
