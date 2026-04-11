# 📑 Orchestr8 -- Complete Documentation Index

> **Universal Runtime Control Plane** -- One spec, four runtimes, one tool.
>
> Version **0.3.0** | License: Proprietary HyperSDK

[![License](https://img.shields.io/badge/license-Proprietary-red)](../LICENSE)

---

## 📖 Table of Contents

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
| Templates | [Templates Guide](TEMPLATES.md) | Generate workload specs from 8 built-in templates (`web-app`, `rest-api`, `database`, `cache`, `worker`, `cron-job`, `ml-training`, `microservice`) | 15 min |
| Shell Completions | [Installation Guide](getting-started/01-Installation.md#-shell-completions) | Tab completion for bash, zsh, fish, PowerShell, elvish | 5 min |

### Intermediate

| Topic | Document | What You Learn | Time |
|-------|----------|----------------|------|
| Workflows | [Intermediate Workflows](tutorials/02-intermediate-workflows.md) | Compose files, migrations, output formats, watch mode | 45 min |
| Workload Schema | [Schema Reference](SCHEMA.md) | Full YAML specification: metadata, build, requirements, runtime, network, persistence, health, config, ingress, scaling | 20 min |
| Cost Analysis | [Cost Estimation](COST.md) | Compare costs across AWS, Azure, GCP, DigitalOcean, Linode | 15 min |

### Advanced

| Topic | Document | What You Learn | Time |
|-------|----------|----------------|------|
| Advanced Features | [Advanced Tutorial](tutorials/03-advanced-features.md) | Policies, AES-256 secrets, drift reconciliation, plugins | 60 min |
| CI/CD Integration | [CI/CD Guide](CICD.md) | GitHub Actions, GitLab CI, Jenkins pipelines | 30 min |

---

## 📗 Guides

### CLI Guide

| Guide | Description |
|-------|-------------|
| [CLI Reference](guides/cli/CLI-Reference.md) | All 40+ commands with flags, descriptions, and examples |
| [Quick Reference Card](quick-reference/QUICK_REFERENCE.md) | One-page command cheat sheet |

#### Core Lifecycle Commands

| Command | Description |
|---------|-------------|
| `orchestr8 init` | First-time setup wizard |
| `orchestr8 validate` | Validate workload YAML specification |
| `orchestr8 build` | Build workload container image |
| `orchestr8 run [--runtime <rt>]` | Deploy a workload (optional runtime override) |
| `orchestr8 stop <name>` | Stop a running workload |
| `orchestr8 status <name>` | Get workload instance status |
| `orchestr8 logs <name> [--follow]` | View workload logs |
| `orchestr8 delete <name>` | Delete a workload instance |
| `orchestr8 list` | List all deployed workloads |
| `orchestr8 exec <name> [cmd]` | Execute a command inside a running workload |
| `orchestr8 port-forward <name> <local:remote>` | Forward local ports to a workload |
| `orchestr8 watch [--runtime <rt>]` | Watch spec file and auto-redeploy on changes |

#### Batch Operations

| Command | Description |
|---------|-------------|
| `orchestr8 deploy <dir> [--runtime <rt>] [--fail-fast] [--dry-run]` | Deploy all workloads from a directory |
| `orchestr8 compose up [file] [--runtime <rt>] [--dry-run]` | Deploy all workloads from a compose file |
| `orchestr8 compose down [file]` | Stop all workloads from a compose file |
| `orchestr8 compose validate [file]` | Validate a compose file |

#### Migration

| Command | Description |
|---------|-------------|
| `orchestr8 migrate <name> <target> [--strategy <s>]` | Migrate workload to a different runtime |
| `orchestr8 migration-advice <name> <target>` | AI-powered migration path recommendations |
| `orchestr8 rollback <name>` | Rollback to the latest snapshot |
| `orchestr8 diff <name>` | Compare spec vs stored vs live state |

#### AI & Analysis

| Command | Description |
|---------|-------------|
| `orchestr8 recommend` | AI-powered runtime recommendation with scoring |
| `orchestr8 profile [--name <n>]` | Workload profiling and optimization recommendations |
| `orchestr8 analyze-logs <name>` | Anomaly and pattern detection in logs |
| `orchestr8 scaling-advice` | Predictive scaling recommendations |
| `orchestr8 compare` | Compare workload across runtimes (cost, capabilities) |
| `orchestr8 affinity recommend <class>` | Runtime affinity for a workload class |
| `orchestr8 affinity matrix` | Full compatibility matrix |

#### Infrastructure & Scheduling

| Command | Description |
|---------|-------------|
| `orchestr8 cost [--provider <p>]` | Multi-cloud cost estimation |
| `orchestr8 config [--show] [--init]` | Show or initialize configuration |
| `orchestr8 template <name> [--output <file>]` | Generate workload from template |
| `orchestr8 env create <name> [--tier <t>]` | Create deployment environment |
| `orchestr8 env promote <workload> <from> <to>` | Promote workload between environments |
| `orchestr8 schedule place <name> [--strategy <s>]` | Schedule workload placement |
| `orchestr8 schedule utilization` | Show runtime utilization |

#### Observability

| Command | Description |
|---------|-------------|
| `orchestr8 tui` | Interactive TUI dashboard |
| `orchestr8 serve [--host <h>] [--port <p>]` | Start API server and web dashboard |
| `orchestr8 metrics` | Export Prometheus metrics |
| `orchestr8 health <name> [--summary]` | View workload health history and uptime |
| `orchestr8 events [--last <n>] [--severity <s>]` | View and filter events |
| `orchestr8 audit [--last <n>] [--workload <w>]` | View audit trail |
| `orchestr8 sla check <workload> --uptime <u>` | Check SLA compliance |

#### Security & Compliance

| Command | Description |
|---------|-------------|
| `orchestr8 secrets create <name>` | Create a new encrypted secret |
| `orchestr8 secrets set <secret> <key> <value>` | Set a key-value pair (AES-256 encrypted) |
| `orchestr8 secrets get <secret> <key>` | Retrieve a decrypted value |
| `orchestr8 secrets list` | List all secrets |
| `orchestr8 secrets audit` | Check rotation status |
| `orchestr8 policy-check [--policy <p>]` | Check workload against policies |
| `orchestr8 drift <name> [--reconcile]` | Detect and reconcile configuration drift |

#### Operations

| Command | Description |
|---------|-------------|
| `orchestr8 backup [--name <n>]` | Backup workload state |
| `orchestr8 restore <backup> [--merge]` | Restore state from backup |
| `orchestr8 list-backups` | List available backups |
| `orchestr8 deps show` | Show dependency graph |
| `orchestr8 deps impact <workload>` | Show impact of stopping a workload |
| `orchestr8 webhook add <name> <url>` | Add webhook notification channel |
| `orchestr8 webhook test <name>` | Send test notification |

#### Health-Aware Orchestration

| Command | Description |
|---------|-------------|
| `orchestr8 orchestrate register <name>` | Register workload for health monitoring |
| `orchestr8 orchestrate status` | Health status of all managed workloads |
| `orchestr8 orchestrate health-check` | Run a single round of health checks |
| `orchestr8 orchestrate watch [--interval <s>]` | Continuous health monitoring loop |
| `orchestr8 orchestrate rolling-update <name>` | Rolling update with configurable replicas |
| `orchestr8 orchestrate reset-circuit <name>` | Reset circuit breaker |

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
| [Operational Runbook](RUNBOOK.md) | Troubleshooting playbooks and recovery procedures |
| [Backup & Restore](BACKUP.md) | State backup, disaster recovery, merge restore |

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
| Cost Estimation | [Cost Guide](COST.md) | Multi-cloud analysis across AWS, Azure, GCP, DigitalOcean, Linode |
| Templates | [Templates Guide](TEMPLATES.md) | 8 built-in templates: web-app, rest-api, database, cache, worker, cron-job, ml-training, microservice |

---

## 🚢 Deployment

| Document | Description |
|----------|-------------|
| [Deployment Guide](DEPLOYMENT.md) | Kubernetes manifests, Helm chart, container image deployment |
| [CI/CD Integration](CICD.md) | GitHub Actions, GitLab CI, Jenkins pipeline recipes |
| [Backup & Restore](BACKUP.md) | State backup, disaster recovery, merge restore |

---

## 📐 Reference

| Document | Description |
|----------|-------------|
| [Workload Spec Schema](SCHEMA.md) | Complete YAML specification reference with all fields |
| [REST API Reference](reference/api/API-Reference.md) | 40+ REST API endpoints, request/response schemas |
| [Prometheus Metrics](METRICS.md) | All exported metrics, labels, and types |
| [Cost Models](COST.md) | Provider-specific cost models and formulas |
| [Templates Catalog](TEMPLATES.md) | Built-in template catalog and customization |
| [Operational Runbook](RUNBOOK.md) | Troubleshooting playbooks and recovery procedures |
| [Web Dashboard](WEBUI.md) | REST API server and browser-based UI |

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
| Hub | [Documentation Hub](README.md) | Landing page with role-based quick access |

### Glossary

| Term | Definition |
|------|-----------|
| **Workload** | A deployable unit described by a YAML spec (`workload.yaml`) |
| **Runtime** | A deployment target: Podman, Kubernetes, KubeVirt, or Metal3 |
| **RuntimeKind** | Enum identifying one of the four supported runtimes |
| **Instance** | A running workload on a specific runtime |
| **Migration** | Moving a workload from one runtime to another |
| **Drift** | Divergence between declared spec and actual live state |
| **Compose** | A multi-workload deployment defined in `orchestr8-compose.yaml` |
| **Plugin** | A third-party runtime extension registered via JSON manifest |
| **Policy** | A set of rules evaluated before deployment is allowed |
| **SLA Target** | Uptime/latency/error-rate thresholds for a workload |
| **Circuit Breaker** | Automatic protection that halts health checks after repeated failures |
| **Blue-Green** | Migration strategy running both versions simultaneously before switching |
| **Rolling** | Migration strategy shifting traffic gradually (25%/50%/75%/100%) |
| **Affinity** | AI-learned runtime preference for a workload class |

### FAQ

**Q: Which runtime should I use?**
A: Run `orchestr8 recommend` for AI-powered scoring, or set `runtime.preferred: auto` in your spec for automatic selection.

**Q: Can I migrate between any two runtimes?**
A: Yes. All 12 runtime-pair combinations are supported (4 source x 3 target).

**Q: Is the spec format compatible with Kubernetes YAML?**
A: The orchestr8 spec is its own format (`apiVersion: orchestr8/v1`). Use `orchestr8 template` to generate specs from common patterns.

**Q: How do I run orchestr8 in CI/CD?**
A: Use `--yes --quiet --json` flags for non-interactive, machine-readable output. See the [CI/CD Guide](CICD.md).

**Q: Where is state stored?**
A: In `~/.orchestr8/` by default. Use `orchestr8 backup` and `orchestr8 restore` for portability.

---

## 🗺️ Learning Paths

### Beginner Path

```
Installation  -->  Quick Start  -->  First Deployment  -->  Quick Reference
```

1. [Install orchestr8](getting-started/01-Installation.md)
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
5. [Cost Estimation](COST.md)

### Advanced Path

```
Advanced Features  -->  Security  -->  Plugins  -->  REST API  -->  CI/CD
```

1. [Advanced Features Tutorial](tutorials/03-advanced-features.md)
2. [Security Guide](features/security.md)
3. [Plugin System](features/plugins.md)
4. [REST API Reference](reference/api/API-Reference.md)
5. [CI/CD Integration](CICD.md)

### Enterprise Path

```
Migration Checklist  -->  Security  -->  Deployment  -->  REST API  -->  Runbook
```

1. [Migration Checklist](guides/operations/MIGRATION_CHECKLIST.md)
2. [Security Guide](features/security.md)
3. [Deployment Guide](DEPLOYMENT.md)
4. [REST API Reference](reference/api/API-Reference.md)
5. [Operational Runbook](RUNBOOK.md)

---

## 📄 License

Proprietary - Copyright (c) 2024-2026 HyperSDK. All rights reserved.
