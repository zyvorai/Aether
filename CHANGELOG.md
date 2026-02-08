# Changelog

All notable changes to Orchestr8 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

### Changed
- Version bumped to 0.3.0
- API serve command now lists 30+ endpoints organized by category
- Test suite expanded: 173 unit tests + 19 integration tests (192 total)
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
