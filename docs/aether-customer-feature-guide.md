# Aether — Feature Guide

> **Universal runtime portability.**

Aether is a universal runtime control plane: one workload spec describes what to run, and Aether deploys it to the right runtime, explains why, and migrates it between runtimes with production strategies. It ships a Rust CLI, an interactive TUI, and a React web dashboard over a shared control plane — think of it as Terraform for where your workloads run, not just what they are.

**4** Runtimes (Podman, K8s, KubeVirt, Metal3) · **16** Migration paths · **5** Migration strategies · **65+** CLI commands · **3** Interfaces (CLI, TUI, Web)

This is the customer-facing feature reference. A print-ready PDF of the same content sits alongside this file. Generated from the product's actual capabilities.

## Contents

1. [One Spec, Four Runtimes](#1-one-spec,-four-runtimes)
2. [Production Migration Engine](#2-production-migration-engine)
3. [Intent Decision Engine](#3-intent-decision-engine)
4. [Discovery, Assessment & Cloud Exit](#4-discovery,-assessment-cloud-exit)
5. [Operations & Lifecycle](#5-operations-lifecycle)
6. [Observability & Cost Insight](#6-observability-cost-insight)
7. [AI Ops & Autonomy](#7-ai-ops-autonomy)
8. [Security, Policy & Compliance](#8-security,-policy-compliance)
9. [Confidential Computing (Ragnarok)](#9-confidential-computing-(ragnarok))
10. [Integrations & Extensibility](#10-integrations-extensibility)

## 1. One Spec, Four Runtimes

_A single Aether workload spec validates and deploys everywhere — from a laptop container to bare metal._

| Runtime | Best for |
|---|---|
| Podman | Local dev and edge containers |
| Kubernetes | Cluster orchestration, services, scaling |
| KubeVirt | VM isolation and GPU passthrough |
| Metal3 | Bare-metal performance |

- **Universal Workload Spec** — Describe a workload once in the aether/v1 YAML schema — image, resources, ports, persistence, secrets, and intent. — _Stop maintaining four different manifest dialects for the same app._
- **Validate Before Deploy** — Check spec syntax, intent, and per-runtime compatibility before anything ships. — _Catch misconfiguration at author time, not in production._
- **Run Anywhere** — Deploy a spec to Podman, Kubernetes, KubeVirt, or Metal3 — with the runtime auto-selected or pinned by flag. — _The same command targets local dev, clusters, VMs, and bare metal._
- **Podman & Local Dev** — Run containers locally or at the edge through the Podman adapter with the identical spec you ship to prod. — _True dev/prod parity from the first line of YAML._
- **Kubernetes Native** — Generate Deployments, Services, Ingress, HPA, ConfigMaps, and Secrets from one spec via the Kubernetes adapter. — _Full cluster orchestration without hand-writing manifests._
- **KubeVirt VMs** — Deploy the workload as a KubeVirt virtual machine for strong isolation and GPU passthrough. — _Move a container to a VM when you need harder boundaries — no rewrite._
- **Metal3 Bare Metal** — Target bare-metal hosts through the Metal3 adapter for maximum performance. — _Reach dedicated hardware from the same control plane as everything else._

## 2. Production Migration Engine

_Move a running workload between any two runtimes with health gates, connection draining, and automatic rollback._

- **16 Migration Paths** — Migrate across every ordered pair of the four runtimes with a single migrate command. — _Escape a legacy VM or container stack without a re-platforming project._
- **Blue-Green Cutover** — Stand up the target (green) while the source (blue) keeps serving, health-gate it, then cut over. — _Zero-downtime moves that only switch traffic once the target is proven healthy._
- **Rolling & Immediate** — Choose incremental rolling steps, an immediate stop-then-start, or canary — per migration. — _Match the migration risk profile to each workload._
- **Automatic Rollback** — On a failed health gate the engine cleans up the target and restores the source automatically. — _A failed migration leaves you where you started, not half-migrated._
- **Health Gates & Draining** — Exponential-backoff health validation and connection draining (up to a 30s cap) guard every cutover. — _In-flight requests finish before the old instance goes away._
- **Migration Trace** — Stream every phase — snapshot, build-target, health-gate, drain, update-state — to stderr for debugging. — _See exactly where a migration is and why, step by step._
- **AI Migration Advice** — Get an AI assessment of moving a named workload to a target runtime before you commit. — _Understand the risks and gains of a move up front._

> Migration is designed for stateless and container-image portability. Persistent volume data is not automatically carried across runtimes — see caveats.

## 3. Intent Decision Engine

_Score and rank runtimes on cost, performance, reliability, and availability — with reasons, not a black-box pick._

- **Explainable Placement** — aether decide ranks every eligible runtime with per-dimension scores and human-readable reasons. — _Stop guessing which runtime fits — see the math behind the choice._
- **Four-Dimension Scoring** — Each runtime is scored 0–1 on cost, performance, reliability, and availability with configurable weights. — _Tune the ranking to your organization's priorities._
- **Intent Multipliers** — An intent block (goal, SLA, budget, resilience, compliance) reshapes the weights toward your objective. — _Encode 'low-latency' or 'cost-optimized' once and let the engine honor it._
- **Workload Classification** — Specs are auto-classified (stateless, stateful, GPU, bare-metal, batch) to filter ineligible runtimes. — _GPU jobs never get scored onto runtimes that can't run them._
- **Runtime Compare** — Side-by-side comparison of cost, capabilities, and limitations across all four runtimes for a spec. — _One view to justify a placement decision to your team._
- **Intent Debugger (Radar)** — The dashboard renders scores as a radar chart so you can see trade-offs at a glance. — _Turn placement scoring into a picture stakeholders understand._
- **Affinity Learning** — Recommend runtimes per workload class from a learned compatibility matrix and usage statistics. — _Recommendations sharpen as the platform learns your fleet._

## 4. Discovery, Assessment & Cloud Exit

_Connect to existing clusters, inventory their apps, score portability, and generate a cloud-exit plan._

- **Cluster Connections** — Save and test connections to EKS, AKS, GKE, OpenShift, Rancher, Tanzu, k3s, RKE2, Podman, or Compose. — _Point Aether at what you already run — no agents to install first._
- **Application Discovery** — Scan a connected cluster and persist an inventory snapshot of its applications and workloads. — _Get a real map of what's running before you plan a move._
- **Dependency Graph** — Surface the dependency graph for a discovered application across its services and data stores. — _See what breaks together before you migrate anything._
- **Portability Assessment** — Assess an application's migration readiness and complexity, with an optional target-cluster preflight. — _Know which apps are easy wins and which need work._
- **Migration Plan CRD** — Generate a MigrationPlan (aether.zyvor.dev/v1alpha1) with a recommended or overridden strategy. — _Turn an assessment into a reviewable, versioned plan artifact._
- **Cloud Exit Report** — Produce a Cloud Exit Assessment report for a connection in Markdown, JSON, or text. — _A shareable, evidence-backed case for leaving a cloud._
- **Move (Shadow Deploy)** — Mirror images and shadow-deploy an app to a target with no external traffic, then cut over or roll back. — _Rehearse a cross-cluster move safely before flipping DNS._

## 5. Operations & Lifecycle

_Day-2 workload operations — inspect, scale, restart, back up, roll back, and drain nodes from one tool._

- **Logs, Exec & Port-Forward** — Tail logs with follow mode, exec into containers, forward ports, and copy files across runtimes. — _The kubectl toolbox for every runtime, in one CLI._
- **Scale & Restart** — Scale a Kubernetes workload to a target replica count or trigger a rolling restart. — _Routine scaling actions without leaving Aether._
- **Health-Aware Orchestration** — Register workloads for health monitoring, watch at an interval, and trip or reset circuit breakers. — _Continuous health signal that feeds migrations and auto-rollback._
- **Backup & Restore** — Snapshot workload state locally or to a remote endpoint and restore or merge it back. — _State you can recover after a bad change or a lost host._
- **Snapshot Rollback** — Roll a workload back to any listed snapshot version of its stored state. — _Undo a deployment to a known-good point in seconds._
- **Node Maintenance** — Cordon, uncordon, and safely drain Kubernetes nodes for maintenance windows. — _Take hardware offline without evicting workloads by hand._
- **Watch & Auto-Redeploy** — Watch a spec file and auto-redeploy on change, or reconcile a whole directory of specs. — _A fast inner loop that keeps live state matching your files._

## 6. Observability & Cost Insight

_A k9s-level view of the fleet — metrics, events, audit, drift, cost, and SLA — across CLI, TUI, and web._

- **Web Dashboard** — A 19-page React dashboard with SSE real-time updates, command palette (⌘K), and a live runtime-fabric topology graph. — _One pane of glass for the whole runtime fleet._
- **Interactive TUI** — A k9s-style terminal dashboard for real-time monitoring, logs, and workload navigation. — _Full situational awareness without leaving the terminal._
- **Prometheus Metrics** — Export workload and control-plane metrics in Prometheus format for your existing stack. — _Plug Aether straight into Grafana and Alertmanager._
- **Events & Audit Trail** — Filterable event stream by severity plus a tamper-evident audit trail of every action. — _Know what changed, when, by whom — with integrity you can prove._
- **Drift Detection** — Detect configuration drift between spec, stored state, and live runtime — with optional auto-reconcile. — _Silent config drift becomes a visible, fixable signal._
- **Cost Estimation** — Estimate workload cost across AWS, Azure, GCP, DigitalOcean, and Linode with spot/reserved discounts. — _Compare the price of a workload before you place it._
- **SLA Compliance** — Set SLA tiers per workload and check observed uptime, latency, error rate, and restarts against them. — _Turn availability promises into monitored, reportable targets._

## 7. AI Ops & Autonomy

_Ask Zeus, the terminal and dashboard AI assistant, plus an intelligence layer that predicts, heals, and advises._

- **Ask Zeus Copilot** — An interactive AI ops assistant in the terminal and a permanent dashboard rail for natural-language questions. — _Ask the platform what's wrong and what to do about it._
- **Predictive Scaling** — Recommend scaling actions and forecast capacity from observed workload behavior. — _Size ahead of demand instead of reacting to alerts._
- **Self-Healing & Remediation** — An autonomy layer detects anomalies and proposes or runs remediation for failing workloads. — _Common failures get fixed before they page a human._
- **Log Anomaly Analysis** — Analyze a workload's logs for anomalies and recurring patterns. — _Find the needle in noisy logs without a query language._
- **Digital Twin** — Model the fleet as a digital twin to reason about capacity, placement, and change impact. — _Test a decision against a model before touching production._
- **Command Center Briefing** — A narrative fleet briefing summarizing health, intent, SLA, and the next recommended actions. — _Start the day with a plain-language state of the fleet._
- **Autonomous FinOps & SRE** — Agent panels for autonomous placement, FinOps budget enforcement, and SRE reliability workflows. — _Delegate routine cost and reliability toil to guided agents._

> AI features connect to an OpenAI-compatible LLM or the Forge inference gateway; several autonomy panels ship as opt-in Labs previews.

## 8. Security, Policy & Compliance

_Encrypted secrets, enterprise SSO, RBAC, policy gates, and supply-chain evidence built into the control plane._

- **AES-256-GCM Secrets** — Encrypt all secret values at rest with authenticated AES-256-GCM, with a managed rotation lifecycle. — _Secrets stay protected on disk and rotate on schedule._
- **Enterprise SSO** — Authenticate via LDAP, SAML, or OIDC with role mapping from your identity provider. — _Plug Aether into the SSO your organization already runs._
- **RBAC & Audit Export** — Role-based access control across API and UI, with exportable audit logs for compliance. — _Least-privilege access with an exportable paper trail._
- **Policy Gate on Deploy** — Enforce production/development policy sets — or a custom OPA/Rego policy — before a workload ships. — _Non-compliant workloads are blocked at deploy, not audited after._
- **SBOM Export & Verify** — Produce and validate CycloneDX software bills of materials for workloads. — _Supply-chain evidence auditors and customers can verify._
- **TLS Lifecycle** — Serve the API over HTTPS with self-managed certificate regeneration and opt-in auto-renewal. — _Encrypted endpoints without manual cert babysitting._
- **Hardened Control Plane** — API key auth, CORS protection, rate limiting, input validation, atomic state locking, and hardened file permissions. — _Production-safe defaults across the whole surface._

## 9. Confidential Computing (Ragnarok)

_Run and migrate attested, measured workloads on trusted execution environments with sovereign-cloud controls._

- **Measured Image Catalog** — Sign VM disk images into a verified catalog and verify a file hash or launch digest against it. — _Only known, signed images ever launch._
- **Attestation & Trust** — Verify TEE attestation verdicts (SEV-SNP, TDX) and gate placement on the trust result. — _Prove a workload runs on genuine confidential hardware._
- **Tenant Isolation Check** — Validate a workload spec against tenant isolation policy before scheduling. — _Catch isolation violations before a tenant is exposed._
- **GuestKit Offline Inspection** — Inspect a VM image offline in pre-launch, offline-policy, post-shutdown, or attested-repair modes. — _Assurance on a VM's contents without booting it._
- **Confidential Migration** — Plan and track encrypted, attested migration of confidential workloads (confidential blue-green). — _Move sensitive VMs between runtimes without breaking the trust chain._
- **Sovereign Compliance Check** — Check a workload against sovereign-cloud residency and compliance constraints. — _Keep regulated workloads inside the boundaries they require._

> Confidential features require compatible TEE hardware (AMD SEV-SNP or Intel TDX) and a Kata/KubeVirt confidential runtime.

## 10. Integrations & Extensibility

_Slot Aether into your platform — GPU, storage, GitOps, packaging, plugins, edge fleets, and multi-tenant hosting._

- **Forge GPU/AI** — Read live GPU capacity, node inventory, placement recommendations, and cost from the Forge control plane. — _Reason about GPU/AI infrastructure in the same tool as everything else._
- **Atlas Storage** — Provision policy-driven persistent volumes (Ceph, NFS, ZFS) through Atlas with snapshot and clone support. — _Intent-based storage instead of hand-rolled PVCs._
- **GitOps Reconcile** — Bind a Git repo and branch, sync manually or auto-reconcile on a poller in serve mode. — _Declarative, Git-sourced fleet state with drift correction._
- **Helm & Package Deploy** — Export any workload as a Helm chart, or install via Docker images and deb/rpm packages. — _Deliver Aether-managed workloads through your existing pipeline._
- **Plugin System** — Discover and register runtime plugins from manifests to extend Aether beyond the built-in adapters. — _Add new runtimes and behaviors without forking the core._
- **Webhooks & Notifications** — Route severity-filtered notifications to webhook channels with a retry queue and test/flush controls. — _Wire fleet events into Slack, PagerDuty, or any endpoint._
- **Edge Fleet & Federation** — Register edge sites that drain a reconcile queue from the control plane, with fleet federation across clusters. — _Manage disconnected and edge sites from one control plane._
- **Hosted Multi-Tenancy** — A hosted control-plane foundation with tenant registry, switcher, managed upgrades, and billing/metering. — _Run Aether as a multi-tenant service for many teams._

## Getting started

1. **Build & initialize** — Clone the repo, run cargo build --release, then aether init to run the first-time setup wizard.
2. **Author a spec** — Start from a template (aether template rest-api) or an example workload, then aether validate --spec workload.yaml.
3. **Decide & deploy** — Run aether decide --spec workload.yaml --explain to see the ranked runtime, then aether run --spec workload.yaml.
4. **Migrate with confidence** — Move a live workload with aether migrate my-app kubevirt --strategy blue-green, with rollback on failure.
5. **Open the dashboard** — Run aether serve and browse http://localhost:5090 for the web UI, Command Center, and Ask Zeus copilot.

> **Good to know:** Aether's portability guarantees are strongest for stateless and container-image workloads: migration rebuilds the app image for the target runtime and updates workload state atomically, but persistent volume data is not automatically carried across runtimes and blue-green connection draining is capped at ~30s. Integrations are optional and mostly read-only — Forge (GPU) and Atlas (storage) are off unless configured, and the hosted multi-tenant control plane ships today as a foundation with fleet federation still maturing. Confidential computing requires compatible TEE hardware (AMD SEV-SNP / Intel TDX) and a Kata/KubeVirt confidential runtime, and several AI autonomy panels are opt-in Labs previews that depend on an external or Forge-hosted LLM.

---
_Aether is developed by ZyvorAI Labs. Contact **info@zyvor.dev** · Proprietary & Confidential._
