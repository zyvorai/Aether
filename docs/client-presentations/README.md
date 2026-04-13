# Aether Client Presentations

22 slide decks covering business value, architecture, runtime guides, migration, security, operations, and developer experience.

All presentations available as HTML (viewable in browser) and can be printed to PDF.

## Presentation Index

### Business & Executive

| # | Title | Pages | Audience |
|---|-------|-------|----------|
| 01 | Business Value | 8 | C-suite, VP Infrastructure, Engineering Directors |
| 05 | Quick Start PoC | 6 | Pre-sales, Solutions Architects |
| 13 | Cost Estimation | 6 | Finance, Procurement |
| 22 | Competitive Analysis | 6 | Strategy, Architecture Review |

### Architecture & Technical Deep Dives

| # | Title | Pages | Focus |
|---|-------|-------|-------|
| 02 | Technical Architecture | 8 | Core components, data flow, adapter pattern |
| 07 | Developer Experience | 6 | CLI, templates, compose, watch mode |
| 14 | Plugin System | 6 | JSON-RPC IPC, custom runtimes |
| 15 | TUI Dashboard | 6 | Interactive terminal UI |

### Runtime Guides

| # | Title | Pages | Runtime |
|---|-------|-------|---------|
| 08 | Kubernetes & Helm | 6 | Kubernetes |
| 09 | KubeVirt VMs | 6 | KubeVirt |
| 10 | Metal3 Bare Metal | 6 | Metal3 |
| 11 | Podman Local Dev | 6 | Podman |

### Migration & Operations

| # | Title | Pages | Focus |
|---|-------|-------|-------|
| 03 | Migration Strategies | 8 | Immediate, Blue-Green, Rolling |
| 06 | Observability & Monitoring | 6 | Metrics, health, events, cost |
| 12 | CI/CD Integration | 6 | GitHub Actions, GitLab CI, pipelines |
| 16 | Backup & Disaster Recovery | 6 | Backup, restore, snapshots |
| 17 | Workload Scheduling | 6 | Placement, utilization, optimization |
| 18 | Drift Detection | 6 | Spec vs live state, auto-reconcile |
| 20 | Environments | 6 | Dev/staging/prod promotion |

### Security & Compliance

| # | Title | Pages | Focus |
|---|-------|-------|-------|
| 04 | Security & Compliance | 8 | AES-256-GCM, API auth, audit, policies |
| 19 | SLA Compliance | 6 | Uptime targets, latency budgets |
| 21 | Secrets Management | 6 | Encryption, rotation, audit trail |

## Generating PDFs

```bash
cd docs/client-presentations/

# Single presentation
google-chrome --headless --print-to-pdf=01-business-value.pdf \
  --print-to-pdf-no-header --no-margins 01-business-value.html

# All presentations
for f in *.html; do
  google-chrome --headless --print-to-pdf="${f%.html}.pdf" \
    --print-to-pdf-no-header --no-margins "$f"
done
```

## Color Theme

| Element | Color | Hex |
|---------|-------|-----|
| Brand accent | Dark Orange | `#d35400` |
| Dark backgrounds | Deep Navy | `#0f111a` |
| Light backgrounds | Off White | `#fafbfc` |
| Success | Green | `#22c55e` |
| Error | Red | `#ef4444` |
| CTA gradient | Orange to Crimson | `#d35400 → #c0392b` |
