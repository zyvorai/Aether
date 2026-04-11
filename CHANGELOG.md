# Changelog

All notable changes to Orchestr8 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Namespace Override:** New `-n` / `--namespace` CLI flag and `ORCHESTR8_NAMESPACE` environment variable to override the Kubernetes namespace for all kube-based runtimes (Kubernetes, KubeVirt, Metal3). The flag takes precedence over the env var.
- **Kubernetes Volume Mounts:** ConfigMaps, Secrets, and PVCs are now auto-mounted as volumes in pod manifests:
  - ConfigMaps with `mount_path` are mounted as read-only volumes
  - Secrets with `mount_path` are mounted as read-only volumes
  - PVCs are auto-mounted at `/data` when `persistence.enabled: true`
- **Podman Health Checks:** Workload health probes (HTTP, TCP, Exec) are now mapped to native Podman `--health-cmd` flags with configurable interval and start period. Containers use `--restart on-failure:3` for automatic restart resilience.
- **Podman Health-Aware Status:** `orchestr8 status` now reports Podman health check status (healthy/unhealthy/starting), restart counts, and health-aware readiness from `podman inspect`.
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
- **Plugin Module (`plugin`):** Runtime extension system with JSON manifest discovery from `~/.orchestr8/plugins/`, JSON-RPC style protocol for plugin communication, and persistent registry
- **Health Module (`health`):** Workload health history tracking with bounded ring buffer (max 1000 records), uptime calculation, restart tracking, and timeline views
- **Output Formats:** New `--output` flag supporting `table` (default), `json`, `yaml`, and `wide` formats for `list`, `status`, and `health` commands
- **Dry Run Mode:** Global `--dry-run` flag for `run`, `stop`, `delete`, and `migrate` commands shows what would happen without executing
- **Interactive Runtime Selector:** `orchestr8 run` now shows an interactive menu for manual runtime selection when no `--runtime` is specified
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
- **Metal3 Adapter:** MAC address and image URL now read from workload annotations (`orchestr8.io/boot-mac-address`, `orchestr8.io/image-url`, `orchestr8.io/image-checksum-url`) with fallback to placeholders and warnings
- **Secrets:** Added explicit warnings that XOR obfuscation is NOT cryptographically secure, even when a custom key is set via `ORCHESTR8_SECRET_KEY`
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
  - Persistence to `~/.orchestr8/secrets.json`
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
  - Scheduler placement metrics (`orchestr8_scheduler_placements_total`)
  - Health check metrics (`orchestr8_health_checks_total`)
  - Circuit breaker event metrics (`orchestr8_circuit_breaker_events_total`)
  - Orchestrator restart metrics (`orchestr8_orchestrator_restarts_total`)
  - Secret operation metrics (`orchestr8_secret_operations_total`)
  - Event emission metrics (`orchestr8_events_emitted_total`)
  - Environment promotion metrics (`orchestr8_env_promotions_total`)
  - Affinity recommendation metrics (`orchestr8_affinity_recommendations_total`)

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
  - State management (~/.orchestr8/state.json)
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

[0.3.0]: https://github.com/ssahani/orchestr8/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/ssahani/orchestr8/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/ssahani/orchestr8/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ssahani/orchestr8/releases/tag/v0.1.0
