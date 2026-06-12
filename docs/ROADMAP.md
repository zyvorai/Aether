# Aether Roadmap

> Ship vs Roadmap — no inflated claims.

---

## Shipped (Trust Layer)

- Product portability spine (README, PRODUCT.md)
- Migration internals + trace mode
- `aether decide --explain` + API/dashboard explain
- Deployment topologies, fleet doc (honest now vs later)
- Stateful portability guide
- Demo folders + bench harness template
- Security, networking, production, cloud matrix docs
- Ecosystem + product tiers
- **Hubble UI discovery** — API `GET /api/cluster/cilium/hubble`, Fleet pod deep links, `AETHER_HUBBLE_UI_URL` override
- **Fleet overview dashboard** — expandable apps, per-pod observability links, cluster resource browser
- **CloudOS Kubernetes UX** — Applications, Activity Monitor, Security Center, Helm App Store, AI troubleshoot panel, workspace selector, Pro View
- **PacketWolf live bridge** — `/api/ecosystem/packetwolf/*`, Fleet + Security Center cards, `AETHER_PACKETWOLF_URL`
- **Edge agent + federation** — `aether edge-agent`, `/api/fleet/edge/*`, `/api/fleet/federation/*`, Fleet Edge/Placement tabs
- **CycloneDX SBOM** — `aether sbom export|verify`, `GET /api/security/sbom`, signed image catalog API
- **SAML encrypted assertions** — AES-128-CBC + AES-256-GCM + RSA-OAEP via `AETHER_SAML_SP_KEY`; exc/inclusive C14N verification; mock IdP `AETHER_MOCK_IDP_ENCRYPTED=1` / `AETHER_MOCK_IDP_ENCRYPTED_GCM=1`
- **Playwright exec E2E** — `scripts/kind-playwright-fixture.sh`, `cluster-exec-terminal.spec.ts`, CI `dashboard-exec-e2e` job
- **Intelligent Fleet** — anomaly-based placement (PacketWolf), GitOps auto-target via federation, fleet-wide drift API + dashboard
- **Migration depth** — cross-cluster volume replication plans, coordinated fleet migration plans
- **Hosted SaaS foundation** — tenant registry, billing usage API, `X-Aether-Tenant` / `AETHER_TENANT_ID` isolation hooks
- **Phase D** — Stripe billing hooks, tenant API keys + metering, volume replication executor, anomaly remediation APIs
- **Edge agent replay** — offline queue persistence and multi-action executor (`gitops_sync`, workload lifecycle, drift, cluster actions)

---

## Q3–Q4 targets (Roadmap)

| Item | Area | Tag |
|------|------|-----|
- **Hosted SaaS billing integration (Stripe)** — shipped in Phase D
- **Live volume replication executor (CSI/snapshot)** — shipped in Phase D
- **Anomaly auto-remediation workflows** — shipped in Phase D
- **Edge agent offline queue replay** — full action dispatch + local retry queue
- **Dashboard phases 2131–2180** — hub cross-links, tenant switcher, hosted upgrades, reference-cluster-live-verify
- **Dashboard phases 2181–2230** — hosted federation panel, Settings/Fleet/Backups hub links, Playwright `phases-2181-2230-features.spec.ts`
- **SAML enterprise dialects** — RSA-SHA384 verification, Azure AD enveloped-signature mock IdP (`AETHER_MOCK_IDP_SHA384`, `AETHER_MOCK_IDP_AZURE_AD`)
- **Metal3 lab MAC fixture** — `examples/labs/metal3/workload.yaml` annotations + `AETHER_METAL3_BOOT_MAC` / `AETHER_METAL3_IMAGE_URL` env overrides
- **Hosted SaaS federation API** — `GET /api/hosted/federation`, `POST /api/hosted/tenants/:id/federation/plan`
- **Identity & SSO** — OIDC login flow, SAML SP metadata, LDAP auth, Settings Identity panels, `?token=` bootstrap, Helm auth configmap
- **Dashboard phases 2231–2280** — Orchestrator/Edge/Intelligence hub cross-links, Settings→RBAC link, Stripe billing portal UI, deploy helpers (`deploy-reference-ingress.sh`, `deploy-reference-sso.sh`, `deploy-with-ldap.sh`), Playwright `phases-2231-2280-features.spec.ts` + `identity-sso.spec.ts`

---

## AI Infrastructure OS (Product Vision)

Full vision: [VISION-AI-OS.md](VISION-AI-OS.md) · 100-phase backlog: [PHASES-AI-OS.md](PHASES-AI-OS.md)

| Phase | Focus | Status |
|-------|-------|--------|
| **v1** | Command Center, 12-section nav, Copilot sidebar, Runtime Fabric graph, promote Runtime Advisor / Root Cause / Migration / Cost / Fleet intelligence | **Ship (core)** |
| **v2** | Digital Twin, Capacity Forecasting, Security Copilot, Intent Studio, Workload Generator | **Ship (core panels)** |
| **v3** | Autonomous optimization, Self-healing agents, Knowledge Graph | **Ship (core panels)** |
| **v4** | Full AI OS — multi-cloud, intent→infrastructure, autonomous SRE | **Ship (core panels)** |
| **v5** | Autonomous execute — next actions, healer execute, unified agents, Spotlight | **Ship (core panels)** |
| **v6** | Agent execute loop — notifications, migration, GitOps, cost, security, capacity | **Ship (core panels)** |
| **v7** | Intent & Infrastructure — NL intent, deploy pipeline, violations, templates, versioning | **Ship (core panels)** |
| **v8** | Multi-Cloud & Federation — mesh, arbitrage, geo, volume sync, wave planner, PacketWolf | **Ship (core panels)** |
| **v9** | SRE & Reliability — incidents, error budgets, postmortems, on-call, escalation, MTTR | **Ship (core panels)** |
| **v10** | Knowledge & Graph — impact, blast radius, CMDB, snapshots, placement, export | **Ship (core panels)** |
| **v11** | macOS Native OS — tray sparkline, dock badge, notifications, offline cache, deep links | **Ship (core panels)** |
| **v12** | Copilot & LLM — batch confirm, memory, multi-agent, runbooks, policy explain, audit, RBAC | **Ship (core panels)** |
| **v13** | FinOps & Cost — chargeback, spot/RI, anomalies, unit economics, multi-cloud, trends | **Ship (core panels)** |
| **v14** | Security & Compliance — policy apply, SBOM drift, zero-trust, threat hunt, score trend | **Ship (core panels)** |
| **v15** | Platform & Ecosystem — SaaS tenants, plugins, Helm v2, v1 API, autonomous SRE | **Ship (core panels)** |
| **v16** | Lab Graduation — Terraform/Pulumi v2, mobile, IDE, carbon, compliance, graph export | **Ship (core panels)** |
| **v17** | Extensions & Native — chaos, game days, Spotlight, Shortcuts, Live Activity | **Ship (core panels)** |
| **v18** | Production & Trust — scorecard, auth/HA/OPA planes, CI verify manifests | **Ship (core panels)** |
| **v19** | Live Labs & Reference Cluster — kubeconfig gates, kind fixtures, CI pipeline | **Ship (core panels)** |

### v1 deliverables (this sprint)

| Item | Area | Tag |
|------|------|-----|
| Command Center briefing API | `src/intelligence/briefing.rs` | Ship |
| 12-section dashboard navigation | `web/dashboard/` | Ship |
| Copilot permanent sidebar | `web/dashboard/` | Ship |
| Runtime Fabric graph page | `/fabric` | Ship |
| AI Migration Planner wizard | `/migrations` | Ship |
| Fleet Intelligence brief | `/fleet` | Ship |
| Root Cause fleet view (batch troubleshoot) | `/observability` | Ship |
| AI Studio hub (Intent + Advisor + Designer) | `/ai` | Ship |
| Live Activity cards (migration SSE) | `LiveActivityDock` | Ship |
| Tauri macOS shell (tray + ⌘K palette) | `apps/aether-macos/` | Ship |

### v2 deliverables

| Item | Area | Tag |
|------|------|-----|
| Capacity Forecast panel | `/observability` | Ship |
| Cost Intelligence panel | `/cost` | Ship |
| Security Copilot (policy drafts) | `/security` + `GET /api/intelligence/security/policies` | Ship |
| Digital Twin simulator | `/fabric` + `POST /api/intelligence/digital-twin/simulate` | Ship |
| Tauri tray fleet health tooltip | `tray_health_update` invoke | Lab |

### v3 deliverables

| Item | Area | Tag |
|------|------|-----|
| Autonomous Mode panel | `/settings` + `GET /api/intelligence/autonomy/status` | Ship |
| Self-Healing orchestrator preview | `/observability` + `GET /api/intelligence/healer/preview` | Ship |
| Infrastructure Knowledge Graph | `/labs` + `GET /api/intelligence/knowledge-graph` | Ship |
| Agent Status dock (6 agents) | `AgentStatusDock` | Ship |

### v4 deliverables

| Item | Area | Tag |
|------|------|-----|
| Intent → Infrastructure pipeline | `/ai?tab=pipeline` + `POST /api/intelligence/intent-pipeline` | Ship |
| Autonomous SRE runbook | `/observability` + `GET /api/intelligence/sre/runbook` | Ship |
| Multi-cloud posture | `/fleet` + `GET /api/intelligence/multicloud/posture` | Ship |
| Autonomous placement | `/migrations` + `GET /api/intelligence/autonomous/placement` | Ship |

### v5 deliverables

| Item | Area | Tag |
|------|------|-----|
| Command Center next actions | `/` + `GET /api/command-center/next-actions` | Ship |
| Healer execute API | `/observability` + `POST /api/intelligence/healer/execute` | Ship |
| Unified agent registry | `AgentStatusDock` + `GET /api/intelligence/agents/status` | Ship |
| macOS Spotlight bridge | ⌘⇧Space → command palette | Lab |

### v6 deliverables

| Item | Area | Tag |
|------|------|-----|
| Critical issue notifications | `CriticalIssueNotifier` + `GET /api/command-center/notifications` | Ship |
| Autonomous migration execute | `/migrations` + `POST /api/intelligence/evolution/execute` | Ship |
| GitOps agent auto-sync | `/gitops` + `GET/POST /api/intelligence/gitops/agent/*` | Ship |
| Cost agent auto-right-size | `/cost` + `POST /api/intelligence/cost-optimize/apply` | Ship |
| Security agent auto-remediate | `/security` + `POST /api/intelligence/security/remediate` | Ship |
| Capacity agent auto-scale | `/observability` + capacity scale APIs | Ship |
| Tauri critical alert tooltip | `tray_critical_alert` invoke | Lab |

### v7 deliverables

| Item | Area | Tag |
|------|------|-----|
| Natural language intent parser | `/ai?tab=pipeline` + `POST /api/intelligence/intent/nl-parse` | Ship |
| One-click pipeline deploy | `POST /api/intelligence/intent-pipeline/deploy` | Ship |
| Intent violation dashboard | `IntentPlatformPanel` + `GET /api/intelligence/intent/violations` | Ship |
| Intent SLA breaches | `CommandCenterIntentSla` + `GET /api/intelligence/intent/sla-breaches` | Ship |
| Budget enforcement | `/cost` + `POST /api/intelligence/intent/budget/enforce` | Ship |
| Compliance intent gates | deploy pipeline + `POST /api/intelligence/intent/compliance/check` | Ship |
| Intent template library | `/ai?tab=intent` + `GET /api/intelligence/intent/templates` | Ship |
| Multi-workload bundles | `POST /api/intelligence/intent/bundles` | Ship |
| Intent GitOps diff | `/gitops` + `GET /api/intelligence/intent/gitops-diff` | Ship |
| Intent versioning | `GET/POST /api/intelligence/intent/versions/*` | Ship |

### v8 deliverables

| Item | Area | Tag |
|------|------|-----|
| Live federation execute | `/fleet` + `POST /api/intelligence/federation/execute` | Ship |
| Cross-cloud cost arbitrage | `FederationPlatformPanel` + `GET /api/intelligence/multicloud/cost-arbitrage` | Ship |
| Cluster health mesh | `GET /api/intelligence/federation/health-mesh` | Ship |
| Unified edge + cloud fabric | `/fabric` + `GET /api/intelligence/federation/unified-fabric` | Ship |
| Volume replication status | `/migrations` + `GET /api/intelligence/migration/volume-status` | Ship |
| Migration wave planner | `/migrations` + `GET /api/intelligence/migration/wave-plan` | Ship |
| Geo latency placement | `GET /api/intelligence/federation/geo-placement` | Ship |
| Cloud account vault | `GET /api/intelligence/multicloud/cloud-accounts` | Ship |
| Region lock enforcement | `GET /api/intelligence/federation/region-lock` | Ship |
| PacketWolf placement guard | `GET/POST /api/intelligence/federation/packetwolf-guard/*` | Ship |

### v9 deliverables

| Item | Area | Tag |
|------|------|-----|
| SRE runbook scheduler | `SreReliabilityPanel` + `GET /api/intelligence/sre/schedule` | Ship |
| Incident timeline | `GET /api/intelligence/sre/incident-timeline` | Ship |
| On-call integration | `GET/POST /api/intelligence/sre/on-call/*` | Ship |
| Postmortem generator | `GET /api/intelligence/sre/postmortem` | Ship |
| Error budget dashboard | `GET /api/intelligence/sre/error-budgets` | Ship |
| Chaos experiments | `GET/POST /api/intelligence/sre/chaos/*` | Lab |
| Game days planner | `GET /api/intelligence/sre/game-days` | Lab |
| Runbook execute | `POST /api/intelligence/sre/runbook/execute` | Ship |
| Escalation policies | `GET /api/intelligence/sre/escalation` | Ship |
| MTTR tracking | `GET /api/intelligence/sre/mttr` | Ship |

### v10 deliverables

| Item | Area | Tag |
|------|------|-----|
| Interactive graph (pan/zoom + edge filters) | `/labs` + `GET /api/intelligence/graph/interactive` | Ship |
| Impact analysis | `GraphPlatformPanel` + `GET /api/intelligence/graph/impact` | Ship |
| Blast radius scoring | `GET /api/intelligence/graph/blast-radius` | Ship |
| K8s service import | `POST /api/intelligence/graph/import-k8s` | Ship |
| Threat propagation paths | `GET /api/intelligence/graph/threat-paths` | Ship |
| Graph search | ⌘K action + `GET /api/intelligence/graph/search` | Ship |
| Graph snapshots | `GET/POST /api/intelligence/graph/snapshots/*` | Ship |
| CMDB sync | `GET/POST /api/intelligence/graph/cmdb/*` | Ship |
| Graph-based placement | `GET /api/intelligence/graph/placement` | Ship |
| Neo4j / JSON-LD export | `GET /api/intelligence/graph/export` | Lab |

### v11 deliverables

| Item | Area | Tag |
|------|------|-----|
| Tray fleet sparkline | Tauri `tray_sparkline_update` + briefing sync | Ship |
| Live Activity migrations | SSE + `live_activity_update` invoke | Lab |
| Dock badge issue count | `dock_badge_update` + briefing sync | Ship |
| Native notifications | `native_notification_show` + critical notifier | Ship |
| Spotlight index | `GET /api/intelligence/macos/spotlight` | Lab |
| Shortcuts manifest | `GET /api/intelligence/macos/shortcuts` | Lab |
| Menu extras toggles | Tray menu + `GET /api/intelligence/macos/menu-extras` | Lab |
| Offline briefing cache | `GET/POST /api/intelligence/macos/offline-cache` | Ship |
| Universal links `aether://` | Tauri URL scheme + resolve API | Ship |
| Notarized DMG CI | `.github/workflows/macos-dmg.yml` | Ship |
| macOS platform panel | `/settings` `MacOSPlatformPanel` | Ship |

### v12 deliverables

| Item | Area | Tag |
|------|------|-----|
| Batch confirm pending actions | `POST /api/copilot/confirm-batch` + Copilot page | Ship |
| Copilot memory | `GET /api/intelligence/copilot/memory` | Ship |
| Multi-agent routing | `POST /api/intelligence/copilot/route` | Ship |
| LLM provider status | `GET /api/intelligence/copilot/llm-status` | Ship |
| Voice copilot lab | `GET /api/intelligence/copilot/voice-lab` | Lab |
| Runbook author | `POST /api/intelligence/copilot/runbook` | Ship |
| Policy explainer | `POST /api/intelligence/copilot/policy-explain` | Ship |
| Terminal copilot | `aether copilot` CLI REPL | Ship |
| Copilot audit trail | `GET /api/intelligence/copilot/audit` | Ship |
| Copilot RBAC scopes | `GET /api/intelligence/copilot/rbac-scopes` | Ship |
| Copilot platform panel | `/copilot` `CopilotPlatformPanel` | Ship |

### v13 deliverables

| Item | Area | Tag |
|------|------|-----|
| Chargeback automation | `GET /api/intelligence/finops/chargeback` | Ship |
| Spot/preemptible advisor | `GET /api/intelligence/finops/spot-advisor` | Ship |
| Reserved instance planner | `GET /api/intelligence/finops/reserved-planner` | Ship |
| Cost anomaly detection | `GET /api/intelligence/finops/anomalies` | Ship |
| Unit economics | `GET /api/intelligence/finops/unit-economics` | Ship |
| FinOps agent execute | `POST /api/intelligence/finops/execute` | Ship |
| Multi-cloud cost compare | `GET /api/intelligence/finops/multicloud-compare` | Ship |
| Carbon footprint (Lab) | `GET /api/intelligence/finops/carbon` | Lab |
| Budget alerts webhook | `POST /api/intelligence/finops/budget-webhook` | Ship |
| FinOps trends & forecast | `GET /api/intelligence/finops/trends` | Ship |
| FinOps platform panel v2 | `/cost` `FinOpsPlatformPanel` | Ship |

### v14 deliverables

| Item | Area | Tag |
|------|------|-----|
| Policy auto-apply | `POST /api/intelligence/security/policy-apply` | Ship |
| SBOM drift alerts | `GET /api/intelligence/security/sbom-drift` | Ship |
| Confidential fleet dashboard | `GET /api/intelligence/security/confidential-fleet` | Ship |
| Zero-trust rollout wizard | `GET /api/intelligence/security/zero-trust-wizard` | Ship |
| Compliance report (Lab) | `GET /api/intelligence/security/compliance-report` | Lab |
| Secret rotation agent | `POST /api/intelligence/security/rotation-agent` | Ship |
| Image signing enforcement | `GET /api/intelligence/security/image-enforcement` | Ship |
| Threat hunt mode | `POST /api/intelligence/security/threat-hunt` | Ship |
| Sovereign audit log | `GET/POST /api/intelligence/security/sovereign-audit` | Ship |
| Security score trend | `GET /api/intelligence/security/score-trend` | Ship |
| Security platform panel | `/security` `SecurityPlatformPanel` | Ship |

### v15 deliverables

| Item | Area | Tag |
|------|------|-----|
| SaaS tenant dashboard | `GET /api/intelligence/platform/saas-tenants` | Ship |
| Plugin marketplace | `GET /api/intelligence/platform/plugin-marketplace` | Ship |
| Helm AI generator v2 | `POST /api/intelligence/platform/helm-v2` | Ship |
| Terraform export (Lab) | `GET /api/intelligence/platform/terraform-export` | Lab |
| Pulumi bridge (Lab) | `GET /api/intelligence/platform/pulumi-bridge` | Lab |
| Public v1 AI OS API | `/v1/intelligence/*` | Ship |
| Mobile companion (Lab) | `GET /api/intelligence/platform/mobile-companion` | Lab |
| IDE extensions (Lab) | `GET /api/intelligence/platform/ide-extensions` | Lab |
| Community intents (Lab) | `GET /api/intelligence/platform/community-intents` | Lab |
| Autonomous SRE loop | `GET/POST /api/intelligence/platform/autonomous-sre/*` | Ship |
| Ecosystem platform panel | `/platform` `EcosystemPlatformPanel` | Ship |

### v16 deliverables

| Item | Area | Tag |
|------|------|-----|
| Lab graduation overview | `/labs` + `GET /api/intelligence/labs/overview` | Ship |
| Terraform export v2 | `POST /api/intelligence/labs/terraform-export` | Ship |
| Pulumi bridge v2 | `POST /api/intelligence/labs/pulumi-bridge` | Ship |
| Mobile companion PWA | `GET /api/intelligence/labs/mobile-companion` | Ship |
| IDE extension pack | `GET /api/intelligence/labs/ide-extensions` | Ship |
| Community intent import | `POST /api/intelligence/labs/community-intents/import` | Ship |
| Carbon footprint ship | `GET /api/intelligence/labs/carbon` | Ship |
| Compliance report ship | `GET /api/intelligence/labs/compliance-report` | Ship |
| Voice copilot ship | `GET /api/intelligence/labs/voice-copilot` | Ship |
| Graph export ship | `GET /api/intelligence/labs/graph-export` | Ship |
| Labs graduation panel | `/labs` `LabsGraduationPanel` | Ship |

### v17 deliverables

| Item | Area | Tag |
|------|------|-----|
| Extensions graduation overview | `/observability` + `GET /api/intelligence/extensions/overview` | Ship |
| Chaos experiments ship | `GET/POST /api/intelligence/extensions/chaos/*` | Ship |
| Game days ship + execute | `GET/POST /api/intelligence/extensions/game-days/*` | Ship |
| Live Activity ship | `GET /api/intelligence/extensions/live-activity` | Ship |
| Spotlight index ship | `GET /api/intelligence/extensions/spotlight` | Ship |
| Shortcuts manifest ship | `GET /api/intelligence/extensions/shortcuts` | Ship |
| Menu extras ship | `GET /api/intelligence/extensions/menu-extras` | Ship |
| Native extensions bundle | `GET /api/intelligence/extensions/native-bundle` | Ship |
| SRE extensions bundle | `GET /api/intelligence/extensions/sre-bundle` | Ship |
| Extensions graduation panel | `/observability` `ExtensionsGraduationPanel` | Ship |

### v18 deliverables

| Item | Area | Tag |
|------|------|-----|
| Production overview | `/platform` + `GET /api/intelligence/production/overview` | Ship |
| Production scorecard | `GET /api/intelligence/production/scorecard` | Ship |
| Auth plane | `GET /api/intelligence/production/auth-plane` | Ship |
| OPA plane | `GET /api/intelligence/production/opa-plane` | Ship |
| HA plane | `GET /api/intelligence/production/ha-plane` | Ship |
| Durability plane | `GET /api/intelligence/production/durability-plane` | Ship |
| Hosted plane | `GET /api/intelligence/production/hosted-plane` | Ship |
| Edge fleet plane | `GET /api/intelligence/production/edge-fleet` | Ship |
| Post-deploy manifest | `GET /api/intelligence/production/post-deploy-manifest` | Ship |
| CI smoke manifest | `GET /api/intelligence/production/ci-smoke-manifest` | Ship |
| Production trust panel | `/platform` `ProductionTrustPanel` | Ship |

### v19 deliverables

| Item | Area | Tag |
|------|------|-----|
| Live labs overview | `/labs` + `GET /api/intelligence/livelabs/overview` | Ship |
| Reference cluster runner | `GET /api/intelligence/livelabs/reference-runner` | Ship |
| Kind Playwright fixture | `GET /api/intelligence/livelabs/kind-fixture` | Ship |
| Labs live smoke | `GET /api/intelligence/livelabs/live-smoke` | Ship |
| Post-deploy verify | `GET /api/intelligence/livelabs/post-deploy-verify` | Ship |
| Kubernetes live lab | `GET /api/intelligence/livelabs/kubernetes-lab` | Ship |
| Advanced runtime labs | `GET /api/intelligence/livelabs/advanced-runtime-labs` | Ship |
| CI pipeline jobs | `GET /api/intelligence/livelabs/ci-pipeline` | Ship |
| Cluster exec E2E | `GET /api/intelligence/livelabs/cluster-exec` | Ship |
| Confidential lab | `GET /api/intelligence/livelabs/confidential-lab` | Ship |
| Live labs panel | `/labs` `LiveLabsPanel` | Ship |

---

## How to read this doc

- **Ship** = in repo today with docs/tests
- **Roadmap** = architecture documented, not claimed in product UI
- **Lab** = `examples/labs/` only

Update this file when items graduate to Ship.

---

## Related engineering docs

- [NEXT-STEPS.md](NEXT-STEPS.md) — HA/OIDC implementation checklist
- [Trust Layer guides](index.md#trust-and-proof)
