# Aether AI Infrastructure Operating System — Product Vision

> **Describe the outcome. Aether operates the infrastructure.**

Aether evolves from a universal runtime control plane into a **macOS-native AI Infrastructure Operating System** — not competing with K9s, Lens, Rancher, or Portainer on tables and YAML, but on **outcome-driven intelligence**.

---

## Positioning

| Category | Incumbents | Aether AI OS |
|----------|------------|--------------|
| Terminal visibility | K9s | Live fabric graph + AI root cause |
| Cluster management | Rancher | Fleet intelligence + autonomous placement |
| Container UI | Portainer | Workload designer + intent studio |
| Enterprise platform | OpenShift | Security copilot + compliance intent |
| Kubernetes IDE | Lens | Knowledge graph + digital twin (v2+) |

**Core message:** Tell infrastructure what outcome you want. Aether figures out the rest.

---

## Design Language

Inspired by Arc, Linear, Raycast, Warp, Cursor, and Apple System Settings:

- Frosted glass surfaces (`surface-panel`, backdrop blur)
- Live motion (SSE-driven updates, animated graph edges)
- Depth layers (page-frame, floating command palette)
- Graph-based navigation (Runtime Fabric, dependency graph)
- No giant tables as primary UX

---

## Main Navigation (12 Sections)

| Section | Purpose | Primary route | Sub-pages (tabs / drill-down) |
|---------|---------|---------------|-------------------------------|
| **Overview** | Command Center briefing | `/` | Legacy dashboard details |
| **Fleet** | Multi-cluster inventory | `/fleet` | Activity, Intelligence |
| **Fabric** | Runtime topology graph | `/fabric` | — |
| **Workloads** | Deploy & manage | `/workloads` | Applications, Editor, Templates |
| **AI Studio** | Intent & scoring | `/ai` | Copilot, Affinity, Confidential |
| **Migrations** | Plan & execute moves | `/migrations` | Migration planner, Evolution |
| **Observability** | Logs, metrics, events | `/observability` | Health, Events, Metrics, Alerts |
| **Security** | Threats & posture | `/security` | RBAC, Audit, Policy |
| **Cost** | FinOps intelligence | `/cost` | — |
| **GitOps** | Reconciliation | `/gitops` | Drift |
| **Labs** | Experimental AI features | `/labs` | Helm, Compose, OpenAPI |
| **Settings** | Platform configuration | `/settings` | Platform, Envs, Secrets, Plugins |

Legacy URLs remain valid for bookmarks and E2E tests.

---

## Command Center

The default screen replaces traditional dashboards with a **narrative briefing**:

```text
Good morning.

Fleet Health: 96%
Issues Found: 3
Potential Savings: $1,284/month
Migration Opportunities: 7
Predicted Capacity Risk: GPU cluster reaches saturation in 12 days
```

**API:** `GET /api/command-center/briefing` — aggregates context snapshot, FinOps, predictions, evolution, and drift.

---

## AI Copilot (Permanent Sidebar)

Cursor-style right rail — infrastructure agent, not chatbot.

- Natural language: health, drift, cost, migrations, cluster queries
- Tool calling with pending action confirmation
- Context-aware from current workload / fleet state

**API:** `POST /api/copilot/chat`, `POST /api/copilot/troubleshoot`

---

## Runtime Fabric

Signature visualization — live graph:

```text
Application → Runtime → Cluster → Node → CPU / GPU / NIC / Disk
```

**Route:** `/fabric` — interactive SVG graph from workload + cluster inventory.

---

## Shipped Backend → Vision Mapping

| Vision feature | Rust module | API |
|----------------|-------------|-----|
| AI Runtime Advisor | `src/ai/scoring.rs` | `POST /api/ai/recommend` |
| AI Migration Planner | `src/ai/migration.rs` | `GET /api/ai/migration-plan/:name/:target` |
| AI Root Cause | `src/copilot/`, `src/ai/analyzer.rs` | `POST /api/copilot/troubleshoot` |
| Cost Intelligence | `src/intelligence/finops.rs` | `GET /api/intelligence/cost-optimize` |
| Fleet Intelligence | `src/intelligence/predict.rs` | `GET /api/intelligence/predictions` |
| Capacity signals | `src/intelligence/predict.rs` | Predictions horizon fields |
| Security Copilot | `src/intelligence/security.rs` | `GET /api/intelligence/threats` |
| AI Drift | `src/drift.rs` | Drift APIs + dashboard |
| Autonomous Mode | `src/intelligence/healer.rs` | `autonomy` spec + remediation APIs |
| Ops Copilot | `src/copilot/agent.rs` | `POST /api/copilot/chat` |
| Context brain | `src/intelligence/context.rs` | `GET /api/context/snapshot` |

See [AI-PURPOSE.md](guides/decision-engine/AI-PURPOSE.md) for rule-based vs LLM-assisted behavior.

---

## Enterprise Roadmap Phases

### v1 — Intelligence UX (current sprint)

- Command Center briefing API + UI
- 12-section navigation
- Copilot permanent sidebar
- Runtime Fabric graph prototype
- Promote Runtime Advisor, Root Cause, Migration Planner, Cost + Fleet intelligence to first-class surfaces

### v2 — Generative & Predictive

- Digital Twin simulation layer
- Capacity forecasting charts
- Security Copilot (least-privilege policy generation)
- Intent Studio (natural language → spec)
- AI Workload Designer (prompt → full spec)

### v3 — Autonomous Agents

- Autonomous optimization (global toggle)
- Self-healing orchestration
- Migration, Cost, Security, Capacity, SRE, GitOps agents
- Infrastructure Knowledge Graph

### v4 — Full AI OS

- Multi-cloud orchestration
- Intent → Infrastructure pipeline
- Autonomous SRE platform
- Autonomous runtime placement

At v4, Aether competes with future AI-native infrastructure platforms, not legacy cluster UIs.

---

## macOS Native Path

1. **Web dashboard** — ship AI OS UX in `web/dashboard/` (Vite + React)
2. **Tauri wrapper** — `apps/aether-macos/` native shell
3. **macOS exclusives** — Spotlight search bridge, menu bar fleet health, Live Activity cards (Tauri plugins)

---

## Related Docs

- [ROADMAP.md](ROADMAP.md) — ship vs roadmap tracking
- [WEBUI.md](WEBUI.md) — dashboard development
- [FLEET-ARCHITECTURE.md](architecture/FLEET-ARCHITECTURE.md) — multi-cluster
- [AI-PURPOSE.md](guides/decision-engine/AI-PURPOSE.md) — intelligence layer truth table
