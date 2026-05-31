# Aether AI OS — 100-Phase Vision Backlog

> Phases **1–4** shipped as core panels (see [ROADMAP.md](ROADMAP.md)).  
> Phases **5–104** are the long-horizon backlog — grouped into **10 eras** of 10 phases each.  
> Tags: **Ship** = in repo · **Roadmap** = designed · **Lab** = examples only.

---

## Era A — Autonomous Execute (Phases 5–14)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 5 | Command Center next actions | Prioritized ops queue from intelligence | **Ship** |
| 6 | Healer execute API | Autonomous restart/drift with dry-run gate | **Ship** |
| 7 | Unified agent registry | Single `/agents/status` for all 6 agents | **Ship** |
| 8 | macOS Spotlight bridge | ⌘⇧Space → command search | **Ship** |
| 9 | Critical issue notifications | Tray + web toast on briefing severity | **Ship** |
| 10 | Autonomous migration execute | Policy-gated fleet evolution runner | **Ship** |
| 11 | GitOps agent auto-sync | Federation-aware reconcile loop | **Ship** |
| 12 | Cost agent auto-right-size | FinOps recommendations → spec patches | **Ship** |
| 13 | Security agent auto-remediate | Policy apply with confirmation | **Ship** |
| 14 | Capacity agent auto-scale | Predictive HPA/VPA suggestions | **Ship** |

## Era B — Intent & Infrastructure (Phases 15–24)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 15 | Natural language intent | NL → `intent:` block (LLM-assisted) | **Ship** |
| 16 | One-click pipeline deploy | Validate → policy → POST workload | **Ship** |
| 17 | Intent violation dashboard | Reconciliation loop UI | **Ship** |
| 18 | Outcome SLAs in Command Center | Intent SLA breach cards | **Ship** |
| 19 | Budget enforcement agent | `maxMonthlyUsd` auto-actions | **Ship** |
| 20 | Compliance intent gates | Deploy blocked on sovereign violations | **Ship** |
| 21 | Template library from intent | Goal → full exemplar specs | **Ship** |
| 22 | Multi-workload intent bundles | App + DB + cache as one outcome | **Ship** |
| 23 | Intent diff on GitOps sync | Show intent drift separately | **Ship** |
| 24 | Intent versioning | Rollback intent block history | **Ship** |

## Era C — Multi-Cloud & Federation (Phases 25–34)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 25 | Live federation execute | Auto-target cluster on sync | **Ship** |
| 26 | Cross-cloud cost arbitrage | Real-time provider switch suggestions | **Ship** |
| 27 | Cluster health mesh | Peer cluster heartbeat graph | **Ship** |
| 28 | Edge + cloud unified fabric | Edge agents in Runtime Fabric | **Ship** |
| 29 | Volume replication UI | CSI executor progress in migrations | **Ship** |
| 30 | Multi-cluster migration coordinator | Fleet migration wave planner | **Ship** |
| 31 | Geo latency placement | RTT-aware federation scoring | **Ship** |
| 32 | Cloud account linking | AWS/GCP/Azure credential vault | **Ship** |
| 33 | Region lock enforcement | Sovereign + residency automation | **Ship** |
| 34 | PacketWolf placement guard | Auto-avoid anomalous clusters | **Ship** |

## Era D — SRE & Reliability (Phases 35–44)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 35 | SRE runbook scheduler | Cron-generated runbooks | **Ship** |
| 36 | Incident timeline | Audit + events unified feed | **Ship** |
| 37 | On-call integration | PagerDuty/Opsgenie webhooks | **Ship** |
| 38 | Postmortem generator | Auto postmortem from root cause | **Ship** |
| 39 | Error budget dashboard | SLO burn rate visualization | **Ship** |
| 40 | Chaos experiments | Controlled fault injection API | **Lab** |
| 41 | Game days planner | Fleet-wide drill scenarios | **Lab** |
| 42 | Runbook execute | Approved steps → API mutations | **Ship** |
| 43 | Escalation policies | Agent handoff chains | **Ship** |
| 44 | MTTR tracking | Healing action latency metrics | **Ship** |

## Era E — Knowledge & Graph (Phases 45–54)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 45 | Interactive knowledge graph | Pan/zoom, filter by edge kind | **Ship** |
| 46 | Impact analysis | “What breaks if X fails?” | **Ship** |
| 47 | Blast radius scoring | Migration + twin integration | **Ship** |
| 48 | Service dependency import | K8s Service/Ingress auto-graph | **Ship** |
| 49 | Threat propagation paths | Attack path visualization | **Ship** |
| 50 | Graph search | ⌘K graph-aware queries | **Ship** |
| 51 | Historical graph snapshots | Point-in-time topology | **Ship** |
| 52 | CMDB sync | External inventory import | **Ship** |
| 53 | Graph-based placement | Dependency-aware scheduling | **Ship** |
| 54 | Graph export | Neo4j / JSON-LD export | **Lab** |

## Era F — macOS Native OS (Phases 55–64)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 55 | Menu bar fleet sparkline | CPU/health mini chart in tray | **Ship** |
| 56 | Live Activity migrations | macOS Live Activity API | **Lab** |
| 57 | Dock badge issue count | Badges synced from briefing | **Ship** |
| 58 | Native notifications | Critical alerts via UNUserNotification | **Ship** |
| 59 | Spotlight index extension | Index workloads for macOS search | **Lab** |
| 60 | Shortcuts.app actions | Siri Shortcuts for common ops | **Lab** |
| 61 | Touch Bar / Menu extras | Quick agent toggles | **Lab** |
| 62 | Offline dashboard cache | Read-only briefing when API down | **Ship** |
| 63 | Universal Links | `aether://` deep links | **Ship** |
| 64 | Notarized DMG CI | Signed release pipeline | **Ship** |

## Era G — Copilot & LLM (Phases 65–74)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 65 | Copilot tool confirmation UX | Batch approve pending actions | **Ship** |
| 66 | Voice copilot | Speech → infrastructure queries | Lab |
| 67 | Copilot memory | Session + fleet context persistence | **Ship** |
| 68 | Multi-agent copilot | Specialist sub-agents (SRE, FinOps) | **Ship** |
| 69 | LLM intent parsing | Optional OpenAI/Anthropic backend | **Ship** |
| 70 | Copilot runbook author | NL → markdown runbooks | **Ship** |
| 71 | Copilot policy explainer | OPA violation plain English | **Ship** |
| 72 | Copilot in terminal | `aether copilot` TUI mode | **Ship** |
| 73 | Copilot audit trail | All NL actions logged | **Ship** |
| 74 | Copilot RBAC scopes | Role-limited tool access | **Ship** |

## Era H — FinOps & Cost (Phases 75–84)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 75 | Chargeback automation | Owner/project auto-attribution | **Ship** |
| 76 | Spot/preemptible advisor | Workload → spot eligibility | **Ship** |
| 77 | Reserved instance planner | RI/SP recommendation engine | **Ship** |
| 78 | Cost anomaly detection | Spend spike alerts | **Ship** |
| 79 | Unit economics | Cost per request metric | **Ship** |
| 80 | FinOps agent execute | Auto-downsize on schedule | **Ship** |
| 81 | Multi-cloud cost compare | Live AWS/GCP/Azure in pipeline | **Ship** |
| 82 | Carbon footprint | Region carbon-aware placement | Lab |
| 83 | Budget alerts webhook | Slack/email on cap breach | **Ship** |
| 84 | FinOps dashboard v2 | Trend charts + forecasts | **Ship** |

## Era I — Security & Compliance (Phases 85–94)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 85 | Policy auto-apply | Security Copilot → cluster apply | **Ship** |
| 86 | SBOM drift alerts | Image digest changes | **Ship** |
| 87 | Confidential fleet dashboard | Ragnarok attestation fleet view | **Ship** |
| 88 | Zero-trust rollout wizard | Cilium policy staged deploy | **Ship** |
| 89 | Compliance report PDF | SOC2-style export | Lab |
| 90 | Secret rotation agent | Auto-rotate on policy | **Ship** |
| 91 | Image signing enforcement | Block unsigned deploys | **Ship** |
| 92 | Threat hunt mode | PacketWolf deep investigation UI | **Ship** |
| 93 | Sovereign audit log | Region compliance trail | **Ship** |
| 94 | Security score trend | Fleet posture over time | **Ship** |

## Era J — Platform & Ecosystem (Phases 95–104)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 95 | Hosted SaaS multi-tenant UI | Tenant switcher + billing | **Ship** |
| 96 | Plugin marketplace | Discover/install runtime plugins | **Ship** |
| 97 | Helm AI generator v2 | Full chart + values from intent | **Ship** |
| 98 | Terraform export | Intent → TF modules | Lab |
| 99 | Pulumi bridge | Programmatic infra from pipeline | Lab |
| 100 | Public AI OS API | Versioned `/v1/intelligence/*` | **Ship** |
| 101 | Mobile companion | Read-only fleet status app | Lab |
| 102 | IDE extensions | VS Code / Cursor workload designer | Lab |
| 103 | Community intent library | Shared goal templates | Lab |
| 104 | Full autonomous SRE | Closed-loop ops without human gate | **Ship** |

## Era K — Lab Graduation (Phases 105–114)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 105 | Terraform export v2 | Intent → OpenTofu modules + variables | **Ship** |
| 106 | Pulumi bridge v2 | TypeScript/Python programs from pipeline | **Ship** |
| 107 | Mobile companion | PWA manifest + read-only fleet APIs | **Ship** |
| 108 | IDE extensions | VS Code/Cursor install commands + OpenAPI | **Ship** |
| 109 | Community intents | Import/publish shared goal templates | **Ship** |
| 110 | Carbon footprint | Per-workload CO₂e estimates | **Ship** |
| 111 | Compliance report | SOC2-style JSON evidence bundle | **Ship** |
| 112 | Voice copilot | Web Speech API manifest | **Ship** |
| 113 | Graph export | Neo4j Cypher + JSON-LD download | **Ship** |
| 114 | Lab graduation hub | `/api/intelligence/labs/*` overview | **Ship** |

## Era L — Extensions & Native Ship (Phases 115–124)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 115 | Chaos experiments | Controlled fault injection with dry-run/live gate | **Ship** |
| 116 | Game days planner | Fleet-wide drill scenarios | **Ship** |
| 117 | Live Activity migrations | macOS migration progress cards | **Ship** |
| 118 | Spotlight index | Workload deep links for macOS search | **Ship** |
| 119 | Shortcuts manifest | Siri Shortcuts for common ops | **Ship** |
| 120 | Menu extras toggles | Tray agent toggles | **Ship** |
| 121 | Chaos live execute | `AETHER_CHAOS_ENABLED=1` gate | **Ship** |
| 122 | Game day execute | Dry-run scenario runner | **Ship** |
| 123 | Native extensions bundle | Spotlight + Shortcuts + Live Activity | **Ship** |
| 124 | SRE extensions bundle | Chaos catalog + game day summary | **Ship** |

## Era M — Production & Trust (Phases 125–134)

| # | Phase | Focus | Tag |
|---|-------|-------|-----|
| 125 | Production scorecard | Readiness % from auth/HA/OPA/durability checks | **Ship** |
| 126 | Auth plane status | API key, OIDC, SAML, RBAC paths | **Ship** |
| 127 | OPA enforcement plane | Configure + enforce paths manifest | **Ship** |
| 128 | HA backends plane | PostgreSQL + Redis readiness | **Ship** |
| 129 | Durability plane | Remote backup + audit webhook | **Ship** |
| 130 | Hosted SaaS plane | Tenants, Stripe, metering endpoints | **Ship** |
| 131 | Edge fleet plane | Edge agent registry summary | **Ship** |
| 132 | Post-deploy verify manifest | Health + smoke script steps | **Ship** |
| 133 | CI smoke manifest | labs-e2e, Playwright, kind jobs | **Ship** |
| 134 | Production trust hub | `/api/intelligence/production/*` overview | **Ship** |

---

## How to use this doc

1. Pick the next **Ship** candidate from Era A onward.
2. Graduate phases to [ROADMAP.md](ROADMAP.md) when panels + APIs + tests land.
3. Do not mark **Roadmap** items as shipped in product UI.

See also: [VISION-AI-OS.md](VISION-AI-OS.md) · [AI-PURPOSE.md](guides/decision-engine/AI-PURPOSE.md)
