# Changelog

All notable changes to Aether will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed: dashboard
- The Overview listed one incident twice (the event and the operations queue entry).
- Failed Start, Stop, Restart, Delete and bulk actions were not reported; bulk actions always said "completed".
- The SLA page logged a 404 for every workload without a target.
- The AI pipeline showed a spec full of `null`.
- The command palette buried the workload under dozens of unrelated entries.
- `j` and `k` did nothing until a table row had focus.
- Opening the Overview in a browser no longer writes the macOS offline cache.

### Added
- The workload panel's Spec tab shows the last applied revision of a managed workload, with secret values redacted.
- Events can be acknowledged: `POST /api/events/{id}/acknowledge` (Operator or Admin) and a button on the Events page, which already showed unacknowledged counts.

### Fixed: stop, then start or restart, on Podman and Docker
- Start and Restart failed with `name "<workload>" is already in use`, because a stopped container keeps its name (API, CLI `restart` and `run`, self-healing, snapshot rollback). The stopped container Aether created is now renamed aside, removed once its replacement runs, and restored if it does not. A running container, or one Aether did not create, is never moved.
- A failed `update` on Podman or Docker removed the old container first and could leave nothing. The old container is now kept and started again.
- Start replaces the container, as it always rebuilt from the spec: files written inside the old container are not carried over.

### Fixed: status reflects what was observed
- Managed workloads reported `deployed (<runtime>)` and counted as running even when their container had stopped. `GET /api/workloads`, `aether list`, `aether metrics`, the assistant tools and the dashboard now show the last observed condition, `stopped` only for a stop made through Aether, `stopped unexpectedly` otherwise, and `unknown` without a current check. Responses gain `status_detail` and `observed_at`. API clients that matched `deployed` must be updated.
- The Command Center briefing counted never-checked and stopped workloads as healthy and reported no issues for them. `fleet_health_pct` now covers checked workloads only and is `null` without any; the new `fleet` object has the counts.

- **Dashboard console:** sectioned sidebar with pins and filter, an attention-first Overview that includes the operations queue, live pause and event inbox, improved command palette, keyboard tables and a workload side panel, compact headers, clearer loading and error states. Fixes failed requests shown as empty, invisible loading skeletons and one crashed page blocking the others.

### Operations desk
Guide: `docs/guides/operations/OPERATIONS-DESK.md`. Dashboard page `/operations`.
- **Change journal.** Deploy, update, stop, delete and migrate are now written to the audit trail (success and failure, with the actor) from every path: CLI, API, compose, migration and self-healing. Before, the lifecycle operations of managed workloads were not audited.
- **Applied revisions** with the engine's decision: `aether history`, `GET /api/workloads/{name}/revisions`, `/revision-diff`.
- **Rollback re-applies the recorded spec** (`aether rollback --revision N`).
- **Health checks and incidents.** `aether serve` checks managed workloads every 60 s (`AETHER_OBSERVE_SECS`); incidents open after 2 failed checks (`AETHER_INCIDENT_THRESHOLD`), with acknowledge, notes, preceding changes and a Markdown draft: `aether incidents`.
- **`aether changes`, `aether attention`, `aether explain`, `aether preview`, `aether ops-report`** and their API routes.
- **Change holds:** `aether hold`; held workloads are refused (HTTP 409).

### Fixed (behaviour changes)
- Rollback redeployed whatever the spec file contained at that moment, because a snapshot stores only its path. `aether rollback` now uses the recorded revision; `--version N` keeps the old behaviour with a warning, and it is the fallback when no revision exists.
- `aether migrate` panicked in debug builds because `-s` was both `--strategy` and the global `--spec`; `-s` is the strategy there.
- CLI errors with a hint printed only the hint and hid the error itself.
- Webhook, Slack and PagerDuty delivery panicked when triggered inside `aether serve` (blocking HTTP client inside the async runtime).
- MTTR was computed by a formula (`12 + 3.5 × failures`), not measured. It now comes from recorded incidents and is `null` without data; `fleet_avg_mttr_minutes` and `avg_recovery_minutes` are nullable.
- The on-call test reported `sent: true` without sending anything. `webhook test` sent an Info event that channels with the default `warning` filter dropped. Both now deliver to the channel, or say that they did not.
- SLA evaluation assumed 100 ms latency when none was measured, and error budgets showed 0% uptime for workloads that were never checked.
- `PUT /api/workloads/{name}` stopped the instance and ran a new one without removing the old, which fails on Podman because the name is taken.
- A running server did not see workloads changed by the CLI, and its next save could overwrite them.
- `aether update` dropped the stored namespace, context and resource kind; CLI deploys stored a relative spec path that `serve` could not resolve.
- Preview and documentation note that `aether update` on KubeVirt deletes the VM disk (existing behaviour, not changed).
- README no longer calls the audit hash an HMAC: it is an unkeyed SHA-256.

### Dashboard appearance
- New neutral design with light, dark and system themes, chosen from an Appearance menu, applied before first paint and remembered.
- Fixed layout regressions from the Tailwind 4 upgrade: a global reset in `index.html` cancelled all margin and padding utilities, theme variables overrode the radius scale, and warning and text-size utilities were missing.
- Accessibility: contrast, focus rings, control labels, keyboard access to menus, the navigation drawer and scroll regions.
- System fonts only; the dashboard no longer requests Google Fonts.

### Added (round 4)
- **`aether lint --fix` / `--diff`** (`src/specfix.rs`): safe, comment-preserving fixes for `W020` (typo'd key renamed to the suggested key) and `I021` (snake_case alias to camelCase); refuses edits that raise the error count. `POST /api/spec/fix`.
- **`aether spec-policy RULES.yaml`** (`src/specpolicy.rs`): policy as code. Declarative rule file (path with `[*]`, 14 operators including quantity comparison and globs, `when`, severity), findings with source lines, text/json/junit/github output. `examples/policy/org-baseline.yaml`. `POST /api/spec/policy`.
- **`aether spec-lock write|check`** (`src/speclock.rs`): hash-only baseline file, per-section drift report, `--ignore`, exit 2 on drift. `POST /api/spec/lock`, `/api/spec/lock/check`.
- **`aether manifest-check FILES`** (`src/manifestcheck.rs`): kubeconform-style offline validation of Kubernetes manifests against the bundled k8s-openapi types (types, missing required fields, bad base64, unserved apiVersions, `--strict` unknown fields, `--ignore-missing-schemas`, `--skip`, text/json/junit). `POST /api/spec/manifest-check`.
- **`aether spec-ci github|gitlab|pre-commit`** (`src/specci.rs`): ready-to-paste CI/pre-commit configuration. `GET /api/spec/ci/{provider}`. Root `.pre-commit-hooks.yaml` for consumers of the repository.
- B101-B109 are now cross-checked against the kube-linter check list and Polaris security checks (docs/RESEARCH.md round 4).

### Changed (round 4 upgrades)
- **reqwest 0.12 -> 0.13**, no longer blocked: `openidconnect` 4.1 is built with `default-features = false` (no bundled reqwest) and `src/oidc.rs` implements its `AsyncHttpClient` trait for a thin wrapper around the application's reqwest 0.13 client. One reqwest version in the tree; native-tls/OpenSSL no longer linked (rustls with aws-lc-rs is the 0.13 default). The `form` and `query` features are enabled explicitly (0.13 made them opt-in).
- **`serde_yaml` 0.9 (unmaintained) -> `serde_yaml_ng` 0.10** via a Cargo `package` rename; no source changes, golden and spec-tool tests unchanged and green.
- **Dashboard: Tailwind CSS 3 -> 4** (`tailwindcss` + `@tailwindcss/postcss`, autoprefixer removed, `@import 'tailwindcss'` + `@config` for the existing theme). Renamed utilities migrated (`rounded` -> `rounded-sm`, `shadow-sm` -> `shadow-xs`, `backdrop-blur-sm` -> `backdrop-blur-xs`, `outline-none` -> `outline-hidden`, `flex-shrink-0` -> `shrink-0`, `placeholder-*` -> `placeholder:text-*`); a base rule restores the v3 default border colour (gray-200). Not visually checked in a browser.
- Other npm updates (lucide-react 1.79, @playwright/test 1.65); GitHub Actions pins and Docker base images re-verified as the latest releases.

### Added (round 3)
- **Lint output formats** (`src/lintfmt.rs`): `aether lint --format text|json|sarif|junit|github`. SARIF 2.1.0 carries the properties GitHub code scanning requires, with a stable per-finding fingerprint; JUnit XML and GitHub workflow commands escape everything that comes from the spec.
- **Lint suppression and rule catalog:** `--ignore CODE[,CODE]`, inline `# aether-lint: ignore=CODE` (next line or same line) and `ignore-file=CODE`; suppressed counts are reported; `E00x`/`E01x` cannot be suppressed. `aether lint --list-rules` / `--explain CODE`, `GET /api/spec/lint/rules[/{code}]`. A test keeps the catalog in step with the codes the linter emits.
- **Best-practice rules** `B101`-`B109` behind `aether lint --best-practices` (liveness/readiness probes, non-root, read-only root filesystem, privilege escalation, single replica without a disruption budget, missing network policy, floating sidecar image tags, default service account).
- **`aether spec-merge`** (`src/specmerge.rs`): base spec + overlay files (RFC 7386 merge patch) + `--set path=value`, linted, `POST /api/spec/merge`.
- **`aether spec-render`** (`src/specrender.rs`): canonical spec with defaults expanded, secrets redacted, and a sha256 content hash overall and per section (`--hash` for CI), `POST /api/spec/render`.
- CLI tests (`tests/cli_spec_tools.rs`) run the real binary; handler tests cover the new endpoints. Research notes added to `docs/RESEARCH.md`.

### Added
- **`aether lint`** (`src/speclint.rs`): every problem in a spec with a stable code and a source line, instead of `validate`'s first error. Reports unknown keys with a "did you mean" hint (the workload structs silently ignore them), invalid label/annotation keys and values, control characters, duplicate or zero ports, `runtime.preferred` outside `runtime.allow`, scaling and ingress mistakes, requests above limits, and fields the allowed runtimes ignore. Refuses oversized documents (1 MiB), alias bombs (more than 16 aliases, or aliases in a document over 64 KiB), more than 200,000 nodes and nesting over 48. `--deny-warnings`, `--json`.
- **`aether portability`** (`src/portability.rs`): per-field matrix of what Podman, Kubernetes and KubeVirt honour for the user's own spec, with a score per runtime.
- **`aether spec-plan OLD [NEW]`** (`src/specplan.rs`): offline, Terraform-style preview of a spec edit with the effect per runtime (no effect, update in place, rolling restart, replace); `--detailed-exit-code`; secret-looking values redacted; control characters escaped.
- **API:** `POST /api/spec/lint`, `/api/spec/portability`, `/api/spec/plan`.
- **Golden tests** for the Kubernetes Deployment/Service and KubeVirt VirtualMachine renderers (`tests/golden/`, `AETHER_UPDATE_GOLDEN=1` to regenerate), and a test that every shipped example lints without errors or ignored fields.
- Docs: `docs/guides/spec-tools.md`, `docs/RESEARCH.md` (competitive and standards research, ranked ideas).

### Fixed
- **Documented spec keys were silently ignored.** `network.serviceType`, `networkPolicy`, `externalName`, `sessionAffinity`, `externalTrafficPolicy`, `ciliumNetworkPolicy`, `calicoNetworkPolicy`, `build.buildArgs`, `config.configMaps`, `config.envFrom`, `config.secrets[].mountPath`, `ingress.ingressClassName`, `ingress.tlsSecretName` and `ingress.paths[].pathType` and `kubernetes.rolloutStrategy.type` (a `type: Recreate` was ignored) were deserialised from snake_case only, although the schema, docs and shipped examples use camelCase. The structs now use camelCase and keep the snake_case spellings as aliases. Specs that set `serviceType: NodePort` now really render a NodePort Service (it was ClusterIP before).
- `workload-kubevirt.yaml` (root sample) used keys that do not exist in the schema; rewritten to the real schema.
- API routes use axum 0.8 path syntax (`/{name}`); the old `/:name` form panics at router build time on axum 0.8.
- `Dockerfile.aether` runtime stage installs `libssl3t64` (the binary links OpenSSL through native-tls).

### Changed (dependency upgrades)
- **Rust majors:** kube 0.97 -> 5.1 and k8s-openapi 0.23 -> 0.29 (required spec fields are no longer `Option`, jiff timestamps replace chrono, new `GRPCAction.mode` / `Lifecycle.stop_signal` fields; the RFC 3339 `Z` suffix replaces `+00:00` in discovery timestamps), axum 0.7 -> 0.8 (`/{name}` route syntax, `Utf8Bytes` WebSocket text), axum-server 0.7 -> 0.9, tower 0.4 -> 0.5, tower-http 0.5 -> 0.7, ratatui 0.29 -> 0.30 and crossterm 0.28 -> 0.29 (`Backend::Error: Send + Sync + 'static` bound), opentelemetry / sdk / otlp 0.27 -> 0.33 and tracing-opentelemetry 0.28 -> 0.34 (`SdkTracerProvider`, blocking OTLP HTTP client, provider kept for shutdown), jsonwebtoken 9 -> 12 (fallible key constructors, explicit `rust_crypto` backend), rand 0.8 -> 0.10 (`rand::rng`, `RngExt`), redis 0.27 -> 1.8, ldap3 0.11 -> 0.12 (`tls-rustls-ring`, keeping the repo's ring preference), x509-parser 0.16 -> 0.18, which 7 -> 8, dirs 5 -> 7, colored 2 -> 3, indicatif 0.17 -> 0.18, comfy-table 7 -> 8 (`TableStyle` replaces presets/modifiers), prometheus 0.13 -> 0.14, base64 0.22 -> 0.23.
- **RustCrypto 0.11 generation, upgraded together:** sha2 0.10 -> 0.11, aes-gcm 0.10 -> 0.11, aes 0.8 -> 0.9, cbc 0.1 -> 0.2, rsa 0.9 -> 0.10, ed25519-dalek 2 -> 3. Digests no longer implement `LowerHex`, so a small `hexfmt::hex_lower` helper replaces `format!("{:x}", digest)`. SAML/secrets round-trip tests pass.
- **Held back: reqwest 0.12** (0.13 is out). `openidconnect` 4.1 (latest) still takes a `reqwest` 0.12 client in `src/oidc.rs`, so a 0.13 bump does not compile until openidconnect moves. reqwest 0.13 would also switch its default TLS to rustls with aws-lc-rs.
- **Held back: tailwindcss 3 -> 4** in the dashboard. v4 replaces the PostCSS plugin and `tailwind.config.js` and rewrites the `@tailwind` directives; the Aurora glass theme and hex-surface guard cannot be visually verified without a browser.
- **Dashboard npm:** react / react-dom 18 -> 19 (with @types), react-router 7 -> 8, vite 6 -> 8, @vitejs/plugin-react 4 -> 6, vitest 3 -> 5, lucide-react 0.468 -> 1.x, typescript 5.9 -> 7.0, plus in-range updates (codemirror, playwright, postcss, ...). `npm run build`, `tsc` and `npm run test` pass; Playwright e2e was not run (no browser on the build host).
- **Docker:** `Dockerfile.aether` builds on `rust:1.100-slim-trixie` (the previous 1.88 is below the MSRV of several upgraded crates) and runs on `debian:trixie-slim`; `Dockerfile` moves to `debian:trixie-slim` with `libssl3t64` (the Debian 13 package name). Tags verified against Docker Hub.
- **GitHub Actions:** checkout v7, setup-node v7, setup-python 3.14, upload-artifact v7, download-artifact v8, docker/build-push-action v7, setup-buildx v4, login v4, metadata v6, softprops/action-gh-release v3, codecov-action v7 (all verified against each repo's latest release); the archived `actions-rs/toolchain` is replaced by `dtolnay/rust-toolchain@stable`. Node pins 20 -> 24. `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`, `upload-pages-artifact@v5` and `deploy-pages@v5` already track their latest major.
- **Docs toolchain:** `requirements-docs.txt` floors raised to `mkdocs>=1.6.1`, `mkdocs-material>=9.8`.
- **Unchanged on purpose:** `bitnami/kubectl:latest` in the Cilium probe CronJob is a floating tag (not pinned in this change); `apps/aether-macos` already uses Tauri 2.
- `cargo fmt` is applied to files that had drifted from rustfmt.

## [0.4.0] - 2026-09-09

### Changed
- **Apache License 2.0 open source:** Aether-core is fully Apache-2.0. Published at https://github.com/zyvorai/Aether.
- **Ragnarok removed from this repository:** Confidential computing (TEE attestation, measured images, sovereign policy, vault/KBS, confidential migration) lives in the separate Ragnarok product. Embedded `src/ragnarok/`, `/api/confidential/*`, and related CLI/integration call sites are gone. `aether confidential` exits with a clear separate-product message.

### Security
- **Viewer keys can no longer open pod shells:** `/api/cluster/ws/exec` is a WebSocket upgrade over GET, and `Role::Viewer` previously allowed every GET — a read-only key could obtain an interactive shell in any pod in any namespace. `rbac::check_permission` now treats the cluster exec route as a privileged GET requiring Operator or Admin.
- **Local key material is gitignored:** `certs/`, `keys/`, `*.pem`, and `*.key` are excluded (mock-idp fixtures excepted) so TLS and signing keys cannot be committed by accident.

### Added
- **Logs and Shell for discovered pods and VMs:** The Workloads detail panel now offers a Shell for cluster-discovered `Pod`, `Deployment`, `StatefulSet`, `DaemonSet`, `VirtualMachine`, and `VirtualMachineInstance` resources. The exec target is resolved before connecting (preferring a Running pod, e.g. a VM's `virt-launcher-*`) instead of passing the composite `cluster/namespace/name` as a pod name, and the terminal header shows the attached pod.
- **Scaling advice declares its data source:** `GET /api/ai/scaling-advice` returns `metrics_source` (`prometheus` or `synthetic`) and annotates the reason when the fixed demo series is used because `AETHER_PROMETHEUS_URL` is unset or a query failed.
- **KubeVirt vGPU Attachment:** `requirements.gpu.vgpuProfile` attaches mediated vGPU slices (e.g. `nvidia.com/GRID_A100-10C`) instead of VFIO passthrough; the profile becomes the KubeVirt GPU `deviceName`. Validation requires a fully-qualified resource name.
- **KubeVirt Live Migration:** new `kubevirt.liveMigration` spec section renders `evictionStrategy: LiveMigrate` and a masquerade pod-network interface (bridge binding is not migratable). New `aether live-migrate <name> [--watch-timeout N]` command creates a `VirtualMachineInstanceMigration` and watches its phase. Validation rejects `liveMigration` with passthrough GPUs — VFIO pins the VM to its host; use `vgpuProfile`.
- **RBAC API Integration:** API middleware now enforces Admin/Operator/Viewer roles via `RbacStore`. New endpoints: `GET /api/rbac/keys`, `POST /api/rbac/keys`, `POST /api/rbac/keys/revoke`. Falls back to `AETHER_API_KEY` for backward compatibility.
- **NetworkPolicy Manifest Generation:** `kube.rs` now generates Kubernetes NetworkPolicy resources from `spec.network.network_policy` config, supporting `deny_all_ingress`, `deny_all_egress`, `allow_from`, and `allow_to` label selectors.
- **Custom HPA Metrics:** `ScalingMetric` now has an optional `metric_name` field. `MetricType::Custom` generates a `PodsMetricSource` in the HPA manifest, enabling scaling on application-specific metrics.
- **Live Drift Detection:** `DriftDetector::check_live_drift()` queries runtime status for failed state, not-ready conditions, and restart count anomalies.
- **Plugin Log Streaming:** `PluginProtocol` now supports `LogsRequest`/`LogsResponse` messages. Plugins with the `"logs"` capability can stream logs back to the CLI and dashboard.
- **Blue-Green Connection Draining:** `migrate_blue_green()` now implements connection draining capped at 30 seconds and documents K8s Service selector behavior during traffic switching.
- **Scheduler Live Capacity Probing:** `probe_capacities()` queries `runtime.capacity()` for K8s/KubeVirt/Metal3 nodes and falls back to defaults when live data is unavailable.
- **Health Check Loop in API Server:** Background tokio task runs health checks every 30 seconds when `aether serve` is running, keeping health state up to date without manual polling.
- **Email SMTP Notifications:** Real SMTP delivery via the `lettre` crate for `ChannelType::Email` notification channels.
- **Multi-Cluster Tests:** 6 unit tests for namespace/context resolution logic in multi-cluster configurations.
- **Web Dashboard Upgrade (k9s-level):**
  - SSE real-time updates: handlers emit `ServerEvent` after mutations; dashboard updates instantly via `useEventStream` hook
  - WorkloadDetail panel: click workload name for tabbed detail view (Overview/Logs/Drift/Scoring) with Start/Stop/Restart/Delete action buttons
  - LogViewer: auto-polling log viewer with follow mode, filter input, line numbers, color-coded log levels, and copy-to-clipboard
  - Command Palette: Cmd+K / Ctrl+K overlay with fuzzy search across pages, workloads, and actions
  - Intent Debugger: AI scoring visualization with pure SVG radar chart (cost/performance/reliability/availability axes)
  - SSE connection indicator: green/red dot in navbar showing live connection status
  - Keyboard shortcuts: `r` (refresh), `?` (command palette), `Cmd+K` (command palette)
- **New Modules:** `src/rbac.rs` (RBAC key store), `src/gitops.rs` (GitOps reconciliation), `src/helm.rs` (Helm chart management)

### Fixed
- **KubeVirt log/pod resolution:** Cluster logs and related-pod lookups for `VirtualMachine`/`VirtualMachineInstance` now try both `kubevirt.io/vm=` and `vm.kubevirt.io/name=` (KubeVirt versions differ) and prefer a Running virt-launcher pod, instead of silently returning nothing when only one label matched.
- **Dashboard assets no longer block the Rust build:** `web/dashboard/dist/` was committed *and* consumed by `include_str!`, so deleting the generated bundle broke `cargo build`. The directory is now gitignored and `build.rs` builds it on demand, falling back to placeholder stubs when npm is unavailable.
- **`Cargo.lock` is tracked again** for reproducible application builds.
- **Container Memory Format:** Podman/Docker adapters now convert K8s memory format (e.g., `256Mi`) to container-native format (e.g., `256m`).
- **Rootless Cgroup Handling:** Podman/Docker adapters skip `--cpus`/`--memory` flags when running as rootless (euid != 0) to avoid cgroup permission errors.
- **State File Path:** `state.json` now lives in `~/.aether/` (persistent, shared between CLI and systemd service).
- **Lock File Path:** Lock files use `/run/aether/` (root), `$XDG_RUNTIME_DIR/aether/` (user), or `/tmp/aether-<uid>` (fallback) instead of colocating with the state file.

### Changed
- **Intelligence apply/execute endpoints stopped reporting fake success.** Remediation, FinOps cost apply, security remediate, capacity scale, evolution, federation sync, PacketWolf guard, GitOps agent, SRE runbook, and game-day execution previously pushed action strings onto `executed`/`applied` for non-dry-run calls even though nothing was mutated. They now report those actions under `skipped` with an explicit "not implemented" reason; dry-run output is unchanged.
- **CI parity:** `make ci` now runs dashboard unit tests (`dashboard-unit`), and GitHub Actions clippy uses `--all-targets --all-features` to match the Makefile.
- **Docs corrected:** default UI port is 5090 (not 8080), metrics are served at `GET /api/metrics`, the Quick Start example is `examples/demo-webserver.yaml`, and dashboard page/test counts reflect the current tree.
- Test suite expanded from 958 to 1,064 tests (970 lib + 46 bin + 48 integration)

### Added
- **Intent Engine:** Declarative intent-based deployment via `intent:` YAML spec field. Supports goals (low-latency, high-throughput, cost-optimized, balanced), SLA targets (latency, availability), budget caps, resilience levels (best-effort, standard, high), and compliance options (isolation, encryption). Intent drives scoring engine weight adjustments, runtime filtering, and score multipliers. Intent violations tracked in reconciliation loop. New `aether intent` CLI command. 30 new tests added.
- **React Web Dashboard:** Complete rewrite from single embedded HTML to React 18 + TypeScript + Tailwind CSS + Vite application. 19 pages, 12 shared components, top navbar with dropdown groups, metallic zinc-grey dark theme. 40+ API endpoints wired with confirmation dialogs, toast notifications, error boundary, mobile hamburger menu, deploy new workload button, and auto-refresh indicator.
- **Audit Integrity API:** New `GET /api/audit/verify` endpoint that verifies HMAC-SHA256 hashes on all audit events and returns verified/tampered status per event.
- **Structured JSON Logging:** `AETHER_LOG_FORMAT=json` environment variable enables JSON-formatted tracing output for machine-parseable log ingestion.
- **API Rate Limiting:** Concurrency limiting (200 concurrent requests) on the API server via tower middleware to prevent overload.
- **Systemd Unit File:** `packaging/aether.service` with security hardening (NoNewPrivileges, ProtectSystem, ProtectHome, PrivateTmp).
- **Dashboard Features:** Confirmation dialogs for destructive actions, toast notifications for operation feedback, error boundary for graceful failure handling, mobile hamburger menu for responsive layout, deploy new workload button, auto-refresh indicator.
- **Namespace Override:** New `-n` / `--namespace` CLI flag and `AETHER_NAMESPACE` environment variable to override the Kubernetes namespace for all kube-based runtimes (Kubernetes, KubeVirt, Metal3). The flag takes precedence over the env var.
- **Kubernetes Volume Mounts:** ConfigMaps, Secrets, and PVCs are now auto-mounted as volumes in pod manifests:
  - ConfigMaps with `mount_path` are mounted as read-only volumes
  - Secrets with `mount_path` are mounted as read-only volumes
  - PVCs are auto-mounted at `/data` when `persistence.enabled: true`
- **Podman Health Checks:** Workload health probes (HTTP, TCP, Exec) are now mapped to native Podman `--health-cmd` flags with configurable interval and start period. Containers use `--restart on-failure:3` for automatic restart resilience.
- **Podman Health-Aware Status:** `aether status` now reports Podman health check status (healthy/unhealthy/starting), restart counts, and health-aware readiness from `podman inspect`.
- **Alert Rule Evaluation:** The `orchestrate watch` loop now evaluates alert rules against live system metrics (SLA uptime, restart counts, drift, policy violations, secret expiry) each cycle, with per-rule cooldown support.
- **Plugin Runtime IPC:** Plugins now fully implement the `Runtime` trait via stdin/stdout JSON-RPC. The `PluginRuntime` struct handles `build`, `run`, `stop`, `status`, `delete`, and `list` operations with 60-second timeout, capability checking, and structured error reporting.
- **New CLI Commands:**
  - `exec` - Execute a command inside a running workload (Podman, Kubernetes, KubeVirt)
  - `port-forward` - Forward local ports to a running workload with port validation
  - `watch` - Watch spec file and auto-redeploy on changes (with 1s debounce)
  - `compare` - Compare workload suitability and cost across all four runtimes
  - `init` - First-time setup wizard with runtime detection and sample workload generation
  - `compose up/down/validate` - Multi-workload compose file support with dependency ordering
  - `plugin list/discover/register/remove` - Runtime plugin management system
  - `health` - View workload health history, uptime percentage, and timeline
- **Compose Module (`compose`):** Multi-workload compose file with topological dependency resolution (Kahn's algorithm), circular dependency detection, and per-workload runtime overrides
- **Plugin Module (`plugin`):** Runtime extension system with JSON manifest discovery from `~/.aether/plugins/`, JSON-RPC style protocol for plugin communication, and persistent registry
- **Health Module (`health`):** Workload health history tracking with bounded ring buffer (max 1000 records), uptime calculation, restart tracking, and timeline views
- **Output Formats:** New `--output` flag supporting `table` (default), `json`, `yaml`, and `wide` formats for `list`, `status`, and `health` commands
- **Dry Run Mode:** Global `--dry-run` flag for `run`, `stop`, `delete`, and `migrate` commands shows what would happen without executing
- **Interactive Runtime Selector:** `aether run` now shows an interactive menu for manual runtime selection when no `--runtime` is specified
- **TUI Search/Filter:** Press `/` in the TUI dashboard to filter workloads by name or runtime; `Esc` to clear
- **TUI Resource Panel:** Detail panel now shows CPU, memory, storage, and GPU requirements from the workload spec
- **REST API Endpoints:**
  - `GET /api/plugins` - List registered plugins
  - `POST /api/plugins/discover` - Discover plugins from filesystem
  - `GET /api/health/:workload` - Get health summary for a workload
  - `POST /api/compose/validate` - Validate a compose specification
- **Migration Improvements:**
  - `MigrationPlan::new()` constructor with sensible defaults for timing parameters
  - Configurable `shutdown_delay`, `traffic_shift_interval`, `cleanup_delay`, `max_health_retries`, and `health_retry_base_interval`
  - Exponential backoff (base * 2^attempt, capped at 30s) during rolling migration health checks
  - Guard rejecting same-runtime migration with helpful hint
  - Guard rejecting empty workload name
  - Rollback deletion errors now logged instead of silently discarded
- **State Store:** Advisory file locking (`flock`) on state.json to prevent concurrent write corruption
- **Podman Adapter:** 10-minute timeout for podman commands to prevent indefinite hangs
- **Error Suggestions:** Contextual hints appended to common errors (workload not found, podman missing, cluster connection failures, unknown runtime)
- **Help Documentation:** `help-all` updated with all new commands and output modes
- **Integration Tests:** 290+ lines of new tests covering compose, plugin, health, migration guards, and output modes

### Changed
- **Migration Plan:** Replaced struct literal construction with `MigrationPlan::new()` across all call sites for consistent defaults
- **State Store Locking:** Uses blocking `LOCK_EX` (instead of `LOCK_NB`) so concurrent processes wait briefly rather than failing immediately
- **State Store Locking:** Updated from deprecated `AsRawFd` to modern `AsFd` + `AsRawFd` idiom
- **Engine:** `test_engine_default` renamed to `test_engine_construction`, uses `Engine::new()` instead of `Engine::default()`
- **Adapters:** Simplified `unwrap_or_else(|| fn())` to `unwrap_or_else(fn)` in kube, kubevirt, and metal adapters
- **KubeVirt Tests:** Replaced indexed loop with iterator (`enumerate`) per Clippy recommendation
- **Podman Adapter:** `Default::default()` now logs a warning when podman is not available
- **Commands:** Eliminated 5 instances of `state.get(name).unwrap()` via new `get_workload_state()` helper
- **Dashboard:** Detail panel lines initialized via `vec![]` literal instead of repeated `.push()` calls
- **Status Command:** Health recording gated behind `!is_quiet()` so scripting/machine-readable modes skip disk writes
- **Init Command:** Now prints target directory path before writing `workload.yaml`
- **Compare Command:** `engine.decide()` hoisted outside the loop (was called N times with identical result)
- **Main:** Error display uses `Err(anyhow!(""))` instead of `std::process::exit(1)` to allow proper Drop cleanup
- **Exec/Port-Forward:** Rewrote `run_with_timeout` from spin-loop polling to `tokio::process::Command` with `tokio::time::timeout`
- **Runtime Selector:** Prompt text is now dynamic (`"Choice (1-N, ...)"`) instead of hardcoded `"1-4"`
- **Runtime Selector:** Range check uses idiomatic `(1..=len).contains(&n)`
- **Port Forward:** Rejects port 0 for both local and remote ports
- **Output Tests:** Added `OutputModeGuard` (Drop-based cleanup) to prevent global state pollution across parallel tests

### Fixed
- **Orchestrator:** Replace `.expect("just inserted")` with safe `unreachable!()` in `register()`
- **Orchestrator:** Corrupted `circuit_opened_at` timestamps no longer silently reset the cooldown timer; circuit remains Open and requires manual reset
- **Orchestrator:** Clarified history trimming to prevent unbounded growth
- **KubeVirt Adapter:** Replaced hard-coded 5-second sleep with proper DataVolume polling loop (2s interval, 60s timeout)
- **Metal3 Adapter:** MAC address and image URL now read from workload annotations (`aether.io/boot-mac-address`, `aether.io/image-url`, `aether.io/image-checksum-url`) with fallback to placeholders and warnings
- **Secrets:** Added explicit warnings that XOR obfuscation is NOT cryptographically secure, even when a custom key is set via `AETHER_SECRET_KEY`
- **Scheduler:** Over-committed placements now rejected with error instead of logging a warning and proceeding
- **API Handlers:** State reload failure after migration now returns HTTP 500 error instead of silent HTTP 200
- **API Handlers:** `ai_analyze_logs` properly handles serialization errors instead of `unwrap_or_default()`
- **API Handlers:** Orphan cleanup in `start_workload` reuses existing runtime client instead of creating a new one
- **Dashboard:** Removed duplicate `status_badge()` call (copy-paste bug)
- **State Store:** `save()` now uses atomic writes (temp file + rename) to prevent corruption on crash
- **Engine:** `parse_cpu`/`parse_memory` now log warnings when falling back to defaults on bad input
- **Config:** Config parse errors elevated from `warn!` to `error!` level with guidance to fix the file
- **Kubernetes Adapter:** `validate_kube_name()` now called at start of `run()` before creating any resources
- **Podman Adapter:** Container listing now skips entries with empty id/name instead of creating ghost instances
- **Affinity Engine:** Division-by-zero fix: returns 0.0 when no metric data available instead of dividing by `max(1)`
- **Backup:** Added symlink checks in `get_backup_info()` and `delete_backup()` to prevent path traversal attacks
- **Migration:** Blue-green traffic switch now uses configurable delay from `MigrationPlan.validation_delay` instead of hard-coded 10s
- **Cost:** CPU parse failure in cost recommendations now logs warning instead of silently defaulting to 0.0
- **Audit:** `prune()` now parses timestamps to `DateTime` for comparison instead of string comparison, fixing issues with different UTC offset formats (`Z` vs `+00:00`)
- **Kubernetes Adapter:** HPA `scale_target_ref` now targets `Deployment` (apps/v1) instead of `Pod` (v1); Pods cannot be scaled by HPA
- **Kubernetes Adapter:** HPA creation skipped when all metrics fail to parse (empty metrics list would be rejected by K8s)
- **Kubernetes Adapter:** HPA creation skipped when `min_replicas > max_replicas`
- **Kubernetes Adapter:** `validate_kube_name()` moved to `common.rs` for reuse across all CRD-based adapters
- **KubeVirt Adapter:** Added `validate_kube_name()` call in `run()` before creating CRD resources
- **KubeVirt Adapter:** GPU `deviceName` fixed from `vendor.com/vendor` to `vendor.com/gpu` (e.g., `nvidia.com/gpu`)
- **Metal3 Adapter:** Added `validate_kube_name()` call in `run()` before creating CRD resources
- **Metal3 Adapter:** Removed dead `_online` variable and unused `spec` binding in `get_host_status()`
- **Metal3 Adapter:** Guard against `i64` overflow in `parse_memory_to_mb()` for extreme memory values
- **Commands:** Replaced `.expect()` on user-provided tier string with proper `?` error propagation
- **Cost:** NaN-safe sort pushes NaN values to end instead of using `unwrap_or(Equal)`
- **AI Scaling:** Variance clamped to `>= 0.0` before `sqrt()` to prevent NaN from negative floating-point results
- **AI Migration:** Removed unreachable `_ => RiskLevel::Critical` match arm (risk_score already clamped to 5)
- **Scheduler:** Affinity scores clamped to `[0.0, 1.0]` before applying bonus to prevent score corruption

## [0.3.0] - 2026-02-08

### Added
- **Secrets Management (`secrets` module):**
  - Encrypted secret storage with XOR obfuscation (dev mode)
  - Create, set, get, delete, and rotate secrets
  - Access audit logging with timestamps and actors
  - Rotation policy enforcement with configurable intervals
  - Rotation audit alerts (warning/critical severity)
  - Persistence to `~/.aether/secrets.json`
  - CLI commands: `secrets create|set|get|list|audit`
  - API endpoint: `GET /api/secrets`

- **Event & Notification System (`events` module):**
  - Event bus with severity levels (Info, Warning, Error, Critical)
  - 10 event categories (Deployment, Migration, SLA, Drift, Policy, Scaling, Health, Secret, Cost, System)
  - Configurable notification channels (Console, File, Webhook)
  - Alert rules with cooldown periods
  - Event filtering by category, severity, and workload
  - Acknowledge/unacknowledge workflow
  - Event pruning for storage management
  - CLI commands: `events list|summary|channels|rules`
  - API endpoints: `GET /api/events`, `GET /api/events/summary`

- **Workload Scheduler (`scheduler` module):**
  - Intelligent scheduling across 4 runtimes
  - 4 scheduling strategies: CostOptimized, PerformanceOptimized, BinPacking, Balanced
  - 8 constraint types: RequireRuntime, ExcludeRuntime, CoLocate, AntiAffinity, MaxCostPerDay, RequireGpu, RequireBareMetal, Zone
  - Priority levels: Low, Normal, High, Critical
  - Runtime capacity tracking with resource utilization
  - Placement recording and release
  - Optimization suggestions (cost, capacity, balance)
  - CLI commands: `schedule place|utilization|optimize|placements`
  - API endpoints: `GET /api/scheduler/utilization`, `GET /api/scheduler/optimize`

- **Health-Aware Orchestrator (`orchestrator` module):**
  - Workload health monitoring with configurable thresholds
  - Circuit breaker pattern (Closed -> Open -> HalfOpen -> Closed)
  - Automatic restart on health check failures
  - Configurable failure/success thresholds
  - Rolling update simulation
  - Health event audit trail (max 100 entries)
  - Manual circuit breaker reset
  - CLI commands: `orchestrate register|status|summary|rolling-update|reset-circuit`
  - API endpoints: `GET /api/orchestrator/status`, `GET /api/orchestrator/summary`

- **Environment Management (`environments` module):**
  - Multi-environment support (Development, Staging, Production, Custom)
  - Workload promotion workflows (Direct, TierAdjusted, Canary)
  - Automatic resource scaling on tier promotion (e.g., 1 replica dev -> 4 replicas prod)
  - Environment parity validation with diff reporting
  - Environment variables per environment
  - Version bumping on promotion
  - CLI commands: `env create|list|promote|parity`
  - API endpoint: `GET /api/environments`

- **Runtime Affinity Learning (`ai/affinity` module):**
  - Deployment outcome recording and learning
  - Composite affinity scoring (success rate, uptime, latency, error rate)
  - Confidence-weighted recommendations
  - Full compatibility matrix (8 workload classes x 4 runtimes)
  - Incompatibility tracking from failure history
  - Heuristic fallback when no deployment data available
  - CLI commands: `affinity recommend|matrix|stats`
  - API endpoint: `GET /api/affinity/:class`

- **Metrics Expansion:**
  - Scheduler placement metrics (`aether_scheduler_placements_total`)
  - Health check metrics (`aether_health_checks_total`)
  - Circuit breaker event metrics (`aether_circuit_breaker_events_total`)
  - Orchestrator restart metrics (`aether_orchestrator_restarts_total`)
  - Secret operation metrics (`aether_secret_operations_total`)
  - Event emission metrics (`aether_events_emitted_total`)
  - Environment promotion metrics (`aether_env_promotions_total`)
  - Affinity recommendation metrics (`aether_affinity_recommendations_total`)

- **Web Dashboard:**
  - Complete rewrite with 10-tab navigation
  - Tabs: Workloads, Scheduler, Health, Events, SLA, Environments, Secrets, Affinity, Templates, Audit
  - 6 stat cards in header
  - Runtime utilization bars per runtime
  - Health summary with circuit breaker badges
  - Event timeline with severity/category badges
  - Auto-refresh every 10 seconds

- **Integration Tests:**
  - Secrets CRUD and encryption roundtrip test
  - Events emit, filter, and acknowledge test
  - Scheduler placement, constraints, and release test
  - Orchestrator health lifecycle and circuit breaker test
  - Environment promotion and parity validation test
  - Affinity learning and recommendation test
  - Scheduler and orchestrator persistence tests

- **New API Endpoints:**
  - `POST /api/workloads/:name/build` - Trigger workload builds via REST
  - `POST /api/validate` - Validate workload YAML via REST
  - `GET /api/secrets/:name` - Get secret metadata (no raw values exposed)
  - `DELETE /api/secrets/:name` - Delete a secret via REST
  - `GET /api/metrics` - Prometheus metrics export endpoint

- **Safety Improvements:**
  - Removed dangerous `block_on()` Default impls from Kubernetes, KubeVirt, Metal3 adapters
  - Replaced `partial_cmp().unwrap()` with safe fallbacks in affinity scoring
  - Fixed unsafe `file_name().unwrap()` in backup listing
  - Fixed `SystemTime` unwrap in scaling advisor
  - Replaced JSON `as_object_mut().unwrap()` with `if let` guards in Metal3 adapter

### Changed
- Version bumped to 0.3.0
- API serve command now lists 38 endpoints organized by category
- Test suite expanded: 387 unit tests + 19 integration tests (406 total)
- 0 clippy warnings

## [0.2.1] - 2026-02-06

### Fixed
- Cargo.toml version now matches git tag (0.1.0 → 0.2.0)
- Collapsed nested `if` statements in decision engine (clippy warnings)
- Fixed `LogParams` field reassignment after `Default` in Kubernetes adapter
- Removed useless `format!` call in cost estimation display
- Derived `Default` for `ServiceType`, `AccessMode`, and `Screen` enums
- Removed needless borrow in backup restore command
- Implemented `/api/workloads/:name/start` endpoint (was returning 501)
- Replaced hardcoded `"running"` status in API responses with actual runtime state

## [0.2.0] - 2026-02-06

### Added
- **Backup and Restore:**
  - Complete backup system for workload state
  - Create named backups with descriptions
  - List and inspect available backups
  - Full restore or merge modes
  - Automatic backup cleanup utilities
  - JSON-based backup format
  - CLI commands: `backup`, `restore`, `list-backups`
- **Cost Estimation:**
  - Multi-provider cost comparison (AWS, Azure, GCP, DigitalOcean, Linode)
  - Resource-based pricing (CPU, memory, storage)
  - Monthly and hourly cost projections
  - Savings analysis across providers
  - CLI command: `cost --provider <provider>`
  - Detailed cost breakdown per resource type
- **WebUI and REST API:**
  - Modern web dashboard for workload management
  - Real-time statistics and monitoring
  - REST API with 11 endpoints
  - Workload operations (list, create, delete, logs, stop)
  - Cost estimation via API
  - Backup management via API
  - Auto-refreshing dashboard (5-second intervals)
  - Modal log viewer
  - CLI command: `serve --host <host> --port <port>`
  - Embedded dashboard HTML (zero-dependency deployment)
- **Production Workload Templates:**
  - 6 production-ready templates for common use cases
  - Web Application template with auto-scaling and ingress
  - Database template (PostgreSQL) with persistence and backups
  - ML Training template with GPU support and distributed training
  - Redis Cache template with LRU eviction and monitoring
  - Batch Job template with retry mechanism and cleanup
  - Microservice template with service mesh integration
  - Comprehensive template documentation and customization guides
  - Best practices for security, scaling, and resource management
- **CI/CD Integration:**
  - Complete GitHub Actions workflow with multi-stage pipeline
  - GitLab CI pipeline with review apps and scheduled jobs
  - Jenkins declarative pipeline with parallel stages
  - Automated cost analysis on pull requests
  - Pre-deployment backup and automatic rollback
  - Blue-green and rolling deployment strategies
  - Integration test automation
  - Performance benchmarking pipelines
  - Comprehensive CI/CD documentation (800+ lines)
- **Production Examples & Runbooks:**
  - Complete microservices e-commerce application example
  - ML training pipeline with GPU orchestration
  - Production runbook with incident response procedures
  - Deployment workflows and automation scripts
  - Performance tuning and optimization guides
  - Disaster recovery procedures
  - Security incident response playbooks
  - Comprehensive troubleshooting guides
- **Advanced Kubernetes Features:**
  - ConfigMaps and Secrets support
  - Ingress with TLS configuration
  - Horizontal Pod Autoscaling (HPA)
  - Environment variables from ConfigMaps/Secrets
- **Shell Completions:**
  - bash, zsh, fish, powershell, and elvish support
  - New `completions` command
- **Prometheus Metrics:**
  - Comprehensive metrics for workload operations
  - Migration tracking (success rate, duration, rollbacks)
  - Runtime distribution and availability
  - CLI command execution metrics
  - New `metrics` command for Prometheus export
  - Integration examples for Grafana and alerting
- **Developer Experience:**
  - JSON Schema for workload YAML validation
  - IDE autocomplete support (VS Code, JetBrains, Neovim)
  - SCHEMA.md documentation with examples
  - VS Code settings for automatic schema association
- **Infrastructure:**
  - CI/CD workflows (GitHub Actions)
  - Container image publishing to GHCR
  - Multi-platform container builds (amd64, arm64)
  - Enhanced Docker metadata and caching
  - Integration tests (8 new tests)
  - Makefile for common development tasks
  - CONTRIBUTING.md guide
  - Security audit in CI
  - Code coverage reporting
- **Documentation:**
  - METRICS.md - Prometheus metrics guide (340 lines)
  - SCHEMA.md - JSON Schema usage guide (220 lines)
  - BACKUP.md - Backup and restore guide (500 lines)
  - COST.md - Cost estimation guide (480 lines)
  - DEPLOYMENT.md - Complete deployment guide (350 lines)
  - WEBUI.md - WebUI and REST API guide (800 lines)
  - TEMPLATES.md - Template usage and customization guide (650 lines)
  - CICD.md - CI/CD integration guide (800 lines)
  - templates/README.md - Quick reference for all templates (300 lines)
  - Full-featured workload example
  - Grafana dashboard documentation (260 lines)
  - Helm chart README with deployment examples (420 lines)
  - Packaging guide for DEB/RPM (200 lines)
- **Deployment & Distribution:**
  - **Grafana Dashboard**: Pre-built dashboard JSON with 14 panels
    - Overview stats and KPIs
    - Runtime distribution visualization
    - Migration metrics and performance tracking
    - Failure tracking and alerts
    - Import-ready for Grafana 8.0+
  - **Helm Chart**: Complete Kubernetes deployment solution
    - RBAC with ClusterRole and ServiceAccount
    - Configurable persistence (PVC)
    - ServiceMonitor for Prometheus Operator
    - HPA and Ingress support
    - Production-ready defaults
    - Comprehensive values.yaml with 60+ options
  - **Package Distribution**:
    - DEB packages for Debian/Ubuntu (amd64, arm64)
    - RPM packages for Fedora/RHEL/openSUSE (x86_64, aarch64)
    - Automated package building in CI/CD
    - Shell completions auto-installed
    - Repository setup instructions (APT, YUM/DNF)
    - Package verification and testing

### Changed
- Enhanced test suite (11 → 32 tests total)
  - Unit tests: 24 (including backup, cost modules)
  - Integration tests: 8
- Updated Kubernetes adapter to support new features
- Improved documentation with advanced features guide
- CLI tagline updated to "four runtimes"
- README with multiple installation options:
  - Container (Docker/Podman) - recommended
  - Binary download from GitHub Releases
  - Build from source
  - Package managers (DEB/RPM)
  - Helm chart (Kubernetes)
- Release workflow with enhanced container publishing
- Added 5,300+ lines of new documentation
- Code metrics: 5,800+ lines total (Rust)
- Templates: 6 production-ready workload templates
- CI/CD Examples: 3 complete pipeline configurations
- New CLI commands: backup, restore, list-backups, cost, serve
- State management with snapshot capabilities
- REST API with comprehensive workload management
- Embedded web dashboard for browser-based management
- Template library with best practices and customization guides
- Production-ready CI/CD pipelines for major platforms

## [0.1.0] - 2024-01-15

### Added
- **Phase 1: Core Foundation**
  - Universal workload specification (YAML)
  - Podman runtime adapter
  - Runtime trait system
  - Decision engine for automatic runtime selection
  - State management (~/.aether/state.json)
  - CLI with 8 commands (validate, build, run, stop, status, logs, delete, list)
  - Comprehensive tests (9 tests passing)

- **Phase 2: Kubernetes Integration**
  - Full Kubernetes runtime adapter
  - Automatic Pod, Service, and PVC generation
  - Health probes (liveness + readiness)
  - Resource limits (CPU, memory)
  - Complete lifecycle operations
  - 500+ line deployment guide (KUBERNETES.md)

- **Phase 3: TUI Dashboard**
  - Interactive terminal UI
  - Real-time workload monitoring
  - Multi-runtime display (🐳☸️🖥️🖧)
  - Integrated log viewer
  - Vim-style keyboard navigation
  - Auto-refresh every 5 seconds
  - Color-coded status indicators
  - Complete TUI guide (400+ lines)

- **Phase 4: KubeVirt Adapter**
  - VirtualMachine and DataVolume CRD generation
  - VM lifecycle operations
  - GPU passthrough configuration
  - Serial console access (virtctl integration)
  - Network interface setup
  - Dynamic Kubernetes API discovery
  - 500+ line VM deployment guide (KUBEVIRT.md)

- **Phase 5: Metal3 Adapter**
  - BareMetalHost CRD generation
  - BMC integration (IPMI/Redfish)
  - Hardware matching via annotations
  - Server provisioning lifecycle
  - Console access instructions
  - Memory and storage unit conversion
  - Provisioning state monitoring
  - 650+ line bare metal guide (METAL3.md)

- **Phase 6: Migration Engine**
  - Three migration strategies:
    - Immediate: Stop source, start target
    - Blue-Green: Zero downtime with traffic switch
    - Rolling: Gradual traffic shift with validation
  - Automatic rollback on failure
  - Health validation with retries
  - State preservation across migrations
  - Support for all 16 runtime pair combinations
  - Configurable validation delays
  - Detailed migration reporting
  - 550+ line migration guide (MIGRATION.md)

### Documentation
- README.md with comprehensive feature list
- KUBERNETES.md - Kubernetes deployment guide (500+ lines)
- KUBEVIRT.md - VM deployment guide (500+ lines)
- METAL3.md - Bare metal provisioning guide (650+ lines)
- MIGRATION.md - Runtime migration guide (550+ lines)
- TUI.md - Interactive dashboard guide (400+ lines)
- FINAL-SUMMARY.md - Complete project summary (800+ lines)
- Phase deliverables (DELIVERABLES.md, PHASE2-6-DELIVERABLES.md)

### Technical Details
- **Language:** Rust 1.70+
- **Code:** 3,705+ lines
- **Documentation:** 5,900+ lines
- **Tests:** 11/11 passing
- **Compiler Warnings:** 0
- **Runtimes:** 4/4 complete
  - 🐳 Podman - Local containers
  - ☸️ Kubernetes - Orchestrated pods
  - 🖥️ KubeVirt - Virtual machines
  - 🖧 Metal3 - Bare metal servers
- **Migration Paths:** 16 (all runtime pairs)
- **Commands:** 9 (validate, build, run, stop, status, logs, delete, list, migrate)

### Dependencies
- `tokio` - Async runtime
- `kube` - Kubernetes client
- `clap` - CLI framework
- `ratatui` - Terminal UI
- `serde` - Serialization
- `anyhow` - Error handling
- `tracing` - Logging
- Total: 324 dependencies

### Performance
- Binary size: 14MB (release)
- Build time: 27s (release)
- Memory usage: 5-15MB
- TUI refresh: 450ms for 10 workloads

[Unreleased]: https://github.com/zyvorai/Aether/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/zyvorai/Aether/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/zyvorai/Aether/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/zyvorai/Aether/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/zyvorai/Aether/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/zyvorai/Aether/releases/tag/v0.1.0
