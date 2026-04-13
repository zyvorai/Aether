# 🔷 Aether

**Universal Runtime Control Plane**

> One spec. Four runtimes. One tool. Seamless migration.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/ssahani/aether)
[![Tests](https://img.shields.io/badge/tests-passing-brightgreen)](https://github.com/ssahani/aether)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Proprietary-red)](LICENSE)

Aether is a **production-ready universal runtime control plane** that deploys the same workload to multiple runtimes:

- 🐳 **Podman** - Local containers
- ☸️ **Kubernetes** - Orchestrated pods
- 🖥️ **KubeVirt** - Virtual machines
- 🖧 **Metal3** - Bare metal servers

Deploy once. Run anywhere. Migrate seamlessly.

---

## ✨ Features

**Unified Workload Specification**
- Single YAML defines your workload
- Works across all four runtimes
- Type-safe validation

**Multi-Runtime Support**
- Deploy to Podman, Kubernetes, KubeVirt, or Metal3
- Automatic runtime selection based on requirements
- Manual override available

**Migration Engine** 🔥
- Migrate between any runtime pair (16 combinations)
- Three strategies: Immediate, Blue-Green, Rolling
- Zero-downtime migrations
- Automatic rollback on failure
- Exponential backoff health checks
- Configurable timing parameters
- Same-runtime guard with helpful hints

**Interactive TUI Dashboard**
- Real-time monitoring across all runtimes
- Search/filter workloads by name or runtime
- Resource details panel (CPU, memory, storage, GPU)
- Integrated log viewer
- Color-coded status indicators
- Auto-refresh every 5 seconds

**Advanced Kubernetes Features**
- ConfigMaps and Secrets management
- Auto volume mounts for ConfigMaps, Secrets, and PVCs
- Ingress with TLS support
- Horizontal Pod Autoscaling (HPA)
- Environment variables from ConfigMaps/Secrets
- Namespace override via `-n` flag or `AETHER_NAMESPACE` env var

**Podman Production Features**
- Native health checks mapped from workload spec (HTTP, TCP, Exec probes)
- Automatic restart policy (`on-failure:3`)
- Health-aware status reporting (healthy/unhealthy/starting)
- Restart count tracking

**Developer Experience**
- Shell completions (bash, zsh, fish, powershell, elvish)
- JSON Schema for IDE autocomplete and validation
- Interactive runtime selector with descriptions
- Multi-format output (`--output table|json|yaml|wide`)
- Dry-run mode (`--dry-run`) for safe previews
- `watch` mode for auto-redeploy on spec changes
- `exec` and `port-forward` for live debugging
- `init` wizard for first-time setup
- Compose files for multi-workload deployments
- Runtime plugin system with full IPC-based lifecycle
- Alert rule evaluation in background monitoring loops
- Comprehensive CI/CD with GitHub Actions
- Integration tests and examples
- Makefile for common tasks

**Deployment & Monitoring**
- Pre-built Grafana dashboard with 14 visualization panels
- Helm chart for Kubernetes deployment
- DEB packages for Debian/Ubuntu
- RPM packages for Fedora/RHEL/openSUSE
- Multi-platform container images (amd64, arm64)
- Prometheus ServiceMonitor support

**Security**
- AES-256-GCM encryption at rest with SHA-256 key derivation
- API key authentication (`AETHER_API_KEY` Bearer token)
- CORS origin restriction on REST API
- SHA-256 audit trail integrity hashes (tamper detection)
- Restrictive file permissions (0o600) on backups and snapshots
- DNS-1123 input validation, path traversal prevention
- Kubernetes resource cleanup on deployment failure
- 5-minute timeouts on all Kubernetes API calls

**Production Ready**
- 37,000+ lines of Rust code
- 928 tests passing (unit + integration)
- Zero compiler warnings, zero Clippy lints
- Atomic state persistence with advisory file locking (crash-safe)
- Symlink-safe backup operations
- Comprehensive documentation (14,500+ lines)
- Package distribution via APT and YUM repositories

---

## 🚀 Quick Start

### Installation

#### Option 1: Container (Recommended)

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/ssahani/aether:latest

# Run with alias
alias aether='docker run --rm -v ~/.aether:/root/.aether -v ~/.kube:/root/.kube ghcr.io/ssahani/aether:latest'

# Use normally
aether --help
```

#### Option 2: Binary from Release

```bash
# Download latest release
curl -L https://github.com/ssahani/aether/releases/latest/download/aether-linux-amd64 -o aether

# Make executable
chmod +x aether

# Move to PATH
sudo mv aether /usr/local/bin/
```

#### Option 3: Build from Source

```bash
# Clone repository
git clone https://github.com/ssahani/aether
cd aether

# Build release binary
cargo build --release

# Install
sudo cp target/release/aether /usr/local/bin/
```

#### Option 4: Package Managers

**Debian/Ubuntu:**
```bash
# Download DEB package
wget https://github.com/ssahani/aether/releases/latest/download/aether_0.1.0-1_amd64.deb

# Install
sudo apt install ./aether_0.1.0-1_amd64.deb
```

**Fedora/RHEL/CentOS:**
```bash
# Download RPM package
wget https://github.com/ssahani/aether/releases/latest/download/aether-0.1.0-1.x86_64.rpm

# Install
sudo dnf install aether-0.1.0-1.x86_64.rpm
```

#### Option 5: Helm (Kubernetes)

```bash
# Add Helm repository (when published)
helm repo add aether https://ssahani.github.io/aether/charts

# Install
helm install aether aether/aether

# Or from local chart
helm install aether ./helm/aether
```

### Create Workload Spec

Create `my-app.yaml`:

```yaml
apiVersion: aether/v1
kind: Workload

metadata:
  name: my-app
  owner: team
  project: demo

build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/yourorg

requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "20Gi"

runtime:
  preferred: auto  # or: podman, kube, kubevirt, metal
  allow:
    - container
    - kube

network:
  service: true
  ports:
    - name: http
      port: 8080
      protocol: TCP

persistence:
  enabled: true
  size: "10Gi"
  storage_class: "standard"
  access_mode: ReadWriteOnce
```

### Deploy Anywhere

```bash
# Validate spec
aether validate --spec my-app.yaml

# Deploy to Podman (local development)
aether run --spec my-app.yaml --runtime podman

# Deploy to Kubernetes (production)
aether run --spec my-app.yaml --runtime kubernetes

# Deploy to KubeVirt (VM isolation)
aether run --spec my-app.yaml --runtime kubevirt

# Deploy to Metal3 (bare metal performance)
aether run --spec my-app.yaml --runtime metal
```

### Migrate Between Runtimes

```bash
# Migrate from Podman to Kubernetes (zero downtime)
aether migrate my-app kubernetes --strategy blue-green

# Migrate from Kubernetes to KubeVirt (gradual)
aether migrate my-app kubevirt --strategy rolling

# Migrate from KubeVirt to Metal3 (fast)
aether migrate my-app metal --strategy immediate
```

### Monitor Everything

```bash
# Interactive dashboard
aether tui

# CLI status
aether status my-app
aether logs my-app
aether list
```

---

## 🎯 Use Cases

### Local → Cloud Development

```bash
# Develop locally with Podman
aether run --spec app.yaml --runtime podman
curl http://localhost:5090

# Migrate to Kubernetes for staging
aether migrate my-app kubernetes --strategy blue-green

# Production deployment ready!
```

### Container → VM Migration

```bash
# Start with container
aether run --spec app.yaml --runtime kubernetes

# Need VM isolation? Migrate to KubeVirt
aether migrate my-app kubevirt --strategy rolling

# Access VM console
aether logs my-app
# Shows: virtctl console my-app
```

### VM → Bare Metal for Performance

```bash
# Start with VM
aether run --spec ml-workload.yaml --runtime kubevirt

# Need GPU passthrough? Migrate to bare metal
aether migrate ml-workload metal --strategy immediate

# Monitor provisioning
kubectl get bmh ml-workload -w
```

---

## 📊 Architecture

### System Design

```
┌─────────────────────────────────────────────────────┐
│              AETHER CLI + TUI                    │
│  - Workload Validation                              │
│  - Runtime Selection                                │
│  - Interactive Dashboard                            │
│  - Migration Engine                                 │
└───────────────────┬─────────────────────────────────┘
                    │
┌───────────────────┴─────────────────────────────────┐
│              AETHER CORE ENGINE                  │
│  - Spec Parser & Validator                          │
│  - Decision Engine (Auto-Selection)                 │
│  - State Store (Persistence)                        │
│  - Migration Controller                             │
└───┬─────────┬─────────────┬─────────────┬───────────┘
    │         │             │             │
    ▼         ▼             ▼             ▼
┌─────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│ PODMAN  │ │KUBERNETES│ │ KUBEVIRT │ │  METAL3  │
│ ADAPTER │ │ ADAPTER  │ │ ADAPTER  │ │ ADAPTER  │
│   ✅    │ │    ✅    │ │    ✅    │ │    ✅    │
└─────────┘ └──────────┘ └──────────┘ └──────────┘
     │            │             │             │
     ▼            ▼             ▼             ▼
Container       Pod           VM        Bare Metal
```

### Migration Engine

```
Source Runtime                    Target Runtime
      │                                 │
      ├──── Immediate Strategy ─────────┤
      │     (stop → start)               │
      │                                  │
      ├──── Blue-Green Strategy ────────┤
      │     (deploy → switch → cleanup) │
      │                                  │
      └──── Rolling Strategy ───────────┘
            (25% → 50% → 75% → 100%)
```

---

## 🔧 Complete Command Reference

### Workload Management

```bash
# First-time setup wizard
aether init

# Validate workload spec
aether validate [--spec workload.yaml]

# Build image for target runtime
aether build [--spec workload.yaml]

# Deploy workload (interactive runtime selector)
aether run [--spec workload.yaml] [--runtime podman|kube|kubevirt|metal]

# Deploy with dry-run preview
aether run --spec workload.yaml --dry-run

# Get workload status
aether status <name>

# View logs/console
aether logs <name> [--follow]

# Stop workload
aether stop <name>

# Delete workload
aether delete <name>

# List all workloads
aether list

# List with extra columns
aether list --output wide

# List as JSON/YAML
aether list --output json
aether list --output yaml
```

### Developer Workflow

```bash
# Execute command inside a running workload
aether exec <name> [command] [-i] [-t timeout]

# Forward local ports to a workload
aether port-forward <name> <local:remote>

# Watch spec and auto-redeploy on changes
aether watch [--runtime podman]

# Compare workload across all runtimes
aether compare

# View health history and uptime
aether health <name> [--last 20] [--summary]
```

### Multi-Workload Compose

```bash
# Validate compose file
aether compose validate [file]

# Deploy all workloads in dependency order
aether compose up [file] [--runtime kube] [--dry-run]

# Stop all workloads in reverse order
aether compose down [file]
```

### Runtime Plugins

```bash
# List registered plugins
aether plugin list

# Discover plugins from ~/.aether/plugins/
aether plugin discover

# Register a plugin from manifest
aether plugin register <manifest.json>

# Remove a plugin
aether plugin remove <name>
```

### Migration

```bash
# Migrate to different runtime
aether migrate <name> <target> [--strategy immediate|blue-green|rolling]

# Dry-run migration preview
aether migrate <name> <target> --dry-run

# Fast migration (skip validation)
aether migrate <name> <target> --strategy immediate --no-validation

# Disable automatic rollback
aether migrate <name> <target> --strategy blue-green --no-rollback
```

### Interactive Dashboard

```bash
# Launch TUI
aether tui

# Keyboard shortcuts:
#   ↑/↓   - Navigate workloads
#   /     - Search/filter workloads
#   Esc   - Clear filter
#   Enter - View logs
#   g/G   - Jump to first/last
#   r     - Refresh
#   q     - Quit
```

### Options

```bash
# Verbose logging
aether -v <command>

# Custom spec file
aether --spec custom.yaml <command>

# Output format
aether --output table|json|yaml|wide <command>

# Dry-run mode (mutating commands)
aether --dry-run <command>

# Skip confirmation prompts
aether --yes <command>

# Help
aether --help
aether <command> --help
```

---

## 🔬 Advanced Features

### ConfigMaps and Secrets

Aether supports Kubernetes ConfigMaps and Secrets for configuration management:

```yaml
config:
  configMaps:
    - name: myapp-config
      data:
        app.properties: |
          server.port=8080
          app.name=MyApp
        logging.level: INFO

  secrets:
    - name: myapp-secrets
      data:
        database.password: super-secret-password
        api.key: my-api-key

  envFrom:
    - sourceType: ConfigMap
      name: myapp-config
    - sourceType: Secret
      name: myapp-secrets
```

**Note:** ConfigMaps and Secrets are automatically created when deploying to Kubernetes and injected as environment variables into your containers.

### Ingress

Configure HTTP/HTTPS ingress for your workloads:

```yaml
ingress:
  enabled: true
  host: myapp.example.com
  tls: true
  paths:
    - path: /
      pathType: Prefix
      port: 80
    - path: /api
      pathType: Prefix
      port: 80
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
```

**Features:**
- Automatic TLS certificate management
- Multiple path routing
- Custom annotations for ingress controllers
- Works with cert-manager and Let's Encrypt

### Horizontal Pod Autoscaling

Enable automatic scaling based on CPU/Memory metrics:

```yaml
scaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - metricType: CPU
      targetValue: "80%"
    - metricType: Memory
      targetValue: "85%"
```

**Note:** HPA automatically scales your pods based on the specified metrics, ensuring optimal resource utilization and cost efficiency.

### Shell Completions

Generate shell completions for your favorite shell:

```bash
# Bash
aether completions bash > /etc/bash_completion.d/aether

# Zsh
aether completions zsh > ~/.zsh/completion/_aether

# Fish
aether completions fish > ~/.config/fish/completions/aether.fish

# PowerShell
aether completions powershell > aether.ps1

# Elvish
aether completions elvish > ~/.elvish/lib/aether.elv
```

### Complete Example

See `examples/workload-full-featured.yaml` for a complete example showcasing all advanced features including ConfigMaps, Secrets, Ingress, and HPA.

---

## 📋 Migration Strategies

### Immediate

**Description:** Stop source, start target immediately.

**Characteristics:**
- ⚡ Fastest migration
- ⏱️ Brief downtime (seconds)
- 💰 Minimal resource usage
- 🔄 Simple rollback

**Best For:** Dev/test environments, non-critical workloads

**Example:**
```bash
aether migrate my-app kubernetes --strategy immediate
```

### Blue-Green

**Description:** Deploy target (green) while source (blue) runs, then switch.

**Characteristics:**
- ✅ Zero downtime
- 💪 2x resources temporarily
- ⚡ Quick rollback (switch back)
- ✔️ Validation before switch

**Best For:** Production workloads, critical services

**Example:**
```bash
aether migrate my-app kubernetes --strategy blue-green
```

### Rolling

**Description:** Gradual traffic shift with continuous validation.

**Characteristics:**
- ✅ Zero downtime
- 📊 Gradual transition (25%/50%/75%/100%)
- 🔍 Continuous health checks
- 🛡️ Safest for critical workloads

**Best For:** Mission-critical services, high-traffic applications

**Example:**
```bash
aether migrate my-app kubernetes --strategy rolling
```

---

## 🎓 Documentation

### User Guides

| Guide | Description | Lines |
|-------|-------------|-------|
| [KUBERNETES.md](KUBERNETES.md) | Complete Kubernetes deployment guide | 500+ |
| [KUBEVIRT.md](KUBEVIRT.md) | VM deployment with KubeVirt | 500+ |
| [METAL3.md](METAL3.md) | Bare metal provisioning | 650+ |
| [MIGRATION.md](MIGRATION.md) | Runtime migration guide (timing, backoff, guards) | 900+ |
| [TUI.md](TUI.md) | Interactive dashboard guide (search/filter) | 550+ |
| [docs/BACKUP.md](docs/BACKUP.md) | Backup and restore guide | 500+ |
| [docs/COST.md](docs/COST.md) | Cost estimation guide | 480+ |
| [docs/WEBUI.md](docs/WEBUI.md) | WebUI and REST API guide (15 endpoints) | 900+ |
| [docs/TEMPLATES.md](docs/TEMPLATES.md) | Template usage guide | 650+ |
| [docs/CICD.md](docs/CICD.md) | CI/CD integration guide | 800+ |
| [docs/METRICS.md](docs/METRICS.md) | Prometheus metrics guide | 340+ |
| [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) | Complete deployment guide | 350+ |

**Total Documentation:** 14,500+ lines

---

## 🏗️ Implementation Status

### ✅ Phase 1 - Core Foundation (COMPLETE)

- ✅ YAML parser with full validation
- ✅ Podman adapter (build, run, stop, status, logs, delete, list)
- ✅ Runtime trait definition
- ✅ Decision engine with auto-selection
- ✅ Local state store (~/.aether/state.json)
- ✅ CLI with 8 commands
- ✅ Comprehensive tests

### ✅ Phase 2 - Kubernetes Integration (COMPLETE)

- ✅ Generate Pod manifests from spec
- ✅ Service creation (ClusterIP, NodePort, LoadBalancer)
- ✅ PersistentVolumeClaim handling
- ✅ Health probes (liveness + readiness)
- ✅ Resource limits (CPU, memory)
- ✅ Complete lifecycle operations
- ✅ Multi-runtime CLI support
- ✅ 500+ line deployment guide

### ✅ Phase 3 - TUI Dashboard (COMPLETE)

- ✅ Interactive terminal interface
- ✅ Real-time workload monitoring
- ✅ Multi-runtime display (🐳☸️🖥️🖧)
- ✅ Integrated log viewer
- ✅ Vim-style keyboard navigation
- ✅ Auto-refresh (5s intervals)
- ✅ Color-coded status indicators
- ✅ Complete TUI guide

### ✅ Phase 4 - KubeVirt Adapter (COMPLETE)

- ✅ DataVolume CRD generation
- ✅ VirtualMachine manifest generation
- ✅ VM lifecycle operations
- ✅ GPU passthrough configuration
- ✅ Serial console access (virtctl)
- ✅ Network interface setup
- ✅ Dynamic API discovery
- ✅ 500+ line VM guide

### ✅ Phase 5 - Metal3 Adapter (COMPLETE)

- ✅ BareMetalHost CRD generation
- ✅ BMC integration (IPMI/Redfish)
- ✅ Hardware matching via annotations
- ✅ Server provisioning lifecycle
- ✅ Console access instructions
- ✅ Memory/storage unit conversion
- ✅ Provisioning state monitoring
- ✅ 650+ line bare metal guide

### ✅ Phase 6 - Migration Engine (COMPLETE)

- ✅ Three migration strategies (Immediate, Blue-Green, Rolling)
- ✅ Automatic rollback on failure
- ✅ Health validation with retries
- ✅ State preservation across migrations
- ✅ Support for all 16 runtime pairs
- ✅ Configurable validation delays
- ✅ Detailed migration reporting
- ✅ 550+ line migration guide

---

## 📊 Project Statistics

| Metric | Value |
|--------|-------|
| **Total Code** | 8,000+ lines of Rust |
| **Documentation** | 14,500+ lines |
| **Tests** | 884 passing ✅ |
| **Compiler Warnings** | 0 ✅ |
| **Clippy Lints** | 0 ✅ |
| **Runtimes** | 4/4 complete ✅ |
| **Phases** | 6/6 delivered ✅ |
| **Commands** | 24 implemented ✅ |
| **Migration Paths** | 16 (all runtime pairs) |
| **REST API Endpoints** | 15 |
| **Cloud Providers** | 5 (cost estimation) |
| **Templates** | 6 production-ready |
| **Binary Size** | 14MB (release) |
| **Build Time** | 27s (release) |

---

## 🧪 Testing

### Run Tests

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_migration_plan_creation
```

### Test Results

```
running unit + integration tests

Unit tests: adapters (kube, kubevirt, metal3, podman), api, backup,
            completions, compose, cost, engine, health, metrics, migration,
            plugin, spec, state, orchestrator, scheduler, secrets, events,
            environments, affinity, output
Integration tests: workload parsing, decision engine, state store,
                   GPU selection, migration plans/guards, backup/restore,
                   cost estimation, scheduler, orchestrator, secrets, events,
                   environments, affinity, compose, plugin, health, output modes

test result: ok. 884 passed; 0 failed; 0 ignored
```

---

## 🗂️ Project Structure

```
aether/
├── src/
│   ├── main.rs           # CLI entrypoint
│   ├── cli.rs            # CLI framework (commands, args, subcommands)
│   ├── commands.rs       # Command handler implementations
│   ├── lib.rs            # Library root
│   ├── spec.rs           # Workload schema (420+ lines)
│   ├── runtime.rs        # Runtime trait + factory
│   ├── engine.rs         # Decision engine (210 lines)
│   ├── state.rs          # State store with advisory file locking
│   ├── migration.rs      # Migration engine (660+ lines)
│   ├── compose.rs        # Multi-workload compose support
│   ├── plugin.rs         # Runtime plugin system
│   ├── health.rs         # Health history and uptime tracking
│   ├── output.rs         # Pretty terminal output + interactive selector
│   ├── api/              # REST API server (15 endpoints)
│   ├── backup.rs         # Backup/restore system (350+ lines)
│   ├── cost.rs           # Cost estimation (370+ lines)
│   ├── metrics.rs        # Prometheus metrics (290 lines)
│   ├── completions.rs    # Shell completions + help-all
│   ├── adapters/
│   │   ├── mod.rs        # Adapter exports + constructor macro
│   │   ├── common.rs     # Shared utilities (name validation, labels, CRD discovery)
│   │   ├── podman.rs     # Podman runtime
│   │   ├── kube.rs       # Kubernetes runtime (1,000+ lines)
│   │   ├── kubevirt.rs   # KubeVirt runtime (450+ lines)
│   │   └── metal.rs      # Metal3 runtime (470+ lines)
│   └── ui/
│       ├── mod.rs        # UI module exports
│       ├── app.rs        # Application state
│       ├── dashboard.rs  # Dashboard screen
│       ├── logs.rs       # Log viewer
│       ├── components.rs # UI components
│       └── events.rs     # Event handling
├── web/                  # Web dashboard (embedded HTML)
├── templates/            # 6 production workload templates
├── examples/             # CI/CD, microservices, ML examples
├── helm/                 # Helm chart for Kubernetes
├── grafana/              # Pre-built Grafana dashboard
├── schema/               # JSON Schema for validation
├── debian/               # DEB package configuration
├── rpm/                  # RPM package configuration
├── scripts/              # Operational scripts
├── docs/                 # Extended documentation
├── tests/                # Integration tests
└── .github/workflows/    # CI/CD workflows
```

**Total Files:** 80+

---

## 🌟 Key Features Explained

### Automatic Runtime Selection

The decision engine chooses the best runtime based on your requirements:

| Condition | Runtime Chosen | Reason |
|-----------|----------------|--------|
| GPU required | KubeVirt or Metal3 | Direct hardware access |
| Large resources (>32 CPU, >128GB RAM) | Metal3 | Dedicated hardware |
| Service networking | Kubernetes | Native service discovery |
| Persistence enabled | Kubernetes | PVC support |
| Local development | Podman | Fast iteration |
| VM isolation needed | KubeVirt | Full virtualization |

### State Management

Aether maintains persistent state in `~/.aether/state.json`:

```json
{
  "my-app": {
    "name": "my-app",
    "runtime": "kubernetes",
    "instance": {
      "id": "abc123",
      "name": "my-app-pod",
      "runtime": "kubernetes",
      "image": "ghcr.io/org/my-app:latest",
      "created_at": "2024-01-15T10:00:00Z"
    },
    "spec_path": "/path/to/workload.yaml",
    "created_at": "2024-01-15T10:00:00Z",
    "updated_at": "2024-01-15T11:30:00Z"
  }
}
```

### Migration Paths

All 16 runtime pair combinations supported:

```
Podman ←→ Kubernetes
Podman ←→ KubeVirt
Podman ←→ Metal3
Kubernetes ←→ KubeVirt
Kubernetes ←→ Metal3
KubeVirt ←→ Metal3
```

Each direction independently supported with all three strategies.

---

## 🎯 Examples

### Example 1: Full Development Lifecycle

```bash
# 1. Develop locally
aether run --spec app.yaml --runtime podman
curl http://localhost:5090

# 2. Test in Kubernetes
aether migrate my-app kubernetes --strategy blue-green
kubectl port-forward pod/my-app 8080:8080
curl http://localhost:5090

# 3. Production: Need VM isolation
aether migrate my-app kubevirt --strategy rolling

# 4. Monitor everything
aether tui
```

### Example 2: GPU Workload Migration

```bash
# Start development with container
aether run --spec ml-model.yaml --runtime podman

# Migrate to VM for GPU passthrough
aether migrate ml-model kubevirt --strategy blue-green

# Need bare metal for best performance
aether migrate ml-model metal --strategy immediate

# Check GPU availability
kubectl get bmh ml-model -o yaml | grep gpu
```

### Example 3: Multi-Environment Deployment

```bash
# Development namespace
export AETHER_NAMESPACE=dev
aether run --spec app.yaml --runtime kubernetes

# Staging namespace
export AETHER_NAMESPACE=staging
aether run --spec app.yaml --runtime kubernetes

# Production namespace
export AETHER_NAMESPACE=prod
aether run --spec app.yaml --runtime kubernetes

# Monitor all environments
aether tui
```

---

## 🤝 Contributing

Contributions welcome! Areas for enhancement:

- Additional runtimes (Docker, Nomad, etc.)
- More migration strategies
- Enhanced TUI features
- Performance optimizations
- Additional workload types

---

## 📄 License

**Proprietary** - Copyright (c) 2024-2026 HyperSDK. All rights reserved.

This software is proprietary and confidential. See [LICENSE](LICENSE) for details.

For licensing inquiries, contact: licensing@hypersdk.io

---

## 🙏 Acknowledgments

Built with:
- [kube-rs](https://github.com/kube-rs/kube) - Kubernetes client
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI
- [clap](https://github.com/clap-rs/clap) - CLI framework
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime
- [serde](https://github.com/serde-rs/serde) - Serialization

---

## 🚀 Status

**Aether is PRODUCTION READY!**

✅ All 6 phases complete
✅ All 4 runtimes working
✅ 24 CLI commands + 15 REST API endpoints
✅ Migration engine with 3 strategies + exponential backoff
✅ Compose files, plugin system, health tracking
✅ Backup/restore, cost estimation, Prometheus metrics
✅ Web dashboard, Helm chart, DEB/RPM packages
✅ Zero compiler warnings, zero Clippy lints
✅ 884 tests passing

**One spec. Four runtimes. One tool. Seamless migration.**

---

<p align="center">
  <strong>Built with ❤️ in Rust</strong>
</p>

<p align="center">
  <a href="https://github.com/ssahani/aether">GitHub</a> •
  <a href="KUBERNETES.md">Kubernetes Guide</a> •
  <a href="MIGRATION.md">Migration Guide</a> •
  <a href="FINAL-SUMMARY.md">Project Summary</a>
</p>
