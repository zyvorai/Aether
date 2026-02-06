# Changelog

All notable changes to Orchestr8 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD workflows (GitHub Actions)
- Container image for Orchestr8 CLI
- Integration tests (8 new tests)
- Makefile for common development tasks
- CONTRIBUTING.md guide
- Security audit in CI
- Code coverage reporting

### Changed
- Enhanced test suite (11 → 19 tests)
- Improved documentation

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

[Unreleased]: https://github.com/ssahani/orchestr8/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ssahani/orchestr8/releases/tag/v0.1.0
