# 🔷 Orchestr8

**Universal Runtime Control Plane**

> One spec. Three runtimes. One tool.

Orchestr8 is a tri-runtime control plane that can run the same workload as:

- 🐳 **Container (Podman)**
- ☸️ **Kubernetes Pod**
- 🖥️ **KubeVirt VM**
- 🖧 **Bare Metal (Metal3)**

From one Rust TUI + CLI + Engine.

## Quick Start

### 1. Install

```bash
cargo build --release
```

### 2. Create a workload spec

See `workload.yaml` for a complete example.

### 3. Run your workload

```bash
# Validate the spec
orchestr8 validate

# Build the image
orchestr8 build

# Run (auto-selects runtime)
orchestr8 run

# Check status
orchestr8 status my-app

# View logs
orchestr8 logs my-app

# Stop the workload
orchestr8 stop my-app

# Delete the workload
orchestr8 delete my-app

# List all workloads
orchestr8 list

# Launch interactive TUI dashboard
orchestr8 tui
```

## Architecture

```
┌──────────────────────────────────────────────┐
│            ORCHESTR8 CORE ENGINE            │
│  - Spec Parser                               │
│  - Runtime Decision Engine                   │
│  - State Store                               │
├───────────┬──────────┬──────────┬────────────┤
│  PODMAN   │   KUBE   │ KUBEVIRT │   METAL3   │
│  ADAPTER  │  ADAPTER │  ADAPTER │  ADAPTER   │
└───────────┴──────────┴──────────┴────────────┘
```

## Workload Specification

The universal workload spec (`workload.yaml`) is the single source of truth:

```yaml
apiVersion: orchestr8/v1
kind: Workload

metadata:
  name: my-app
  owner: ssahani
  project: demo

build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/yourorg/orchestr8

requirements:
  cpu: 2
  memory: 4Gi
  storage: 20Gi

runtime:
  preferred: auto
  allow:
    - container
    - kube
    - kubevirt

network:
  service: true
  serviceType: ClusterIP
  ports:
    - containerPort: 80
      servicePort: 8080

persistence:
  enabled: true
  size: 10Gi
  accessMode: ReadWriteOnce

health:
  liveness:
    httpGet:
      path: /health
      port: 80
```

## Runtime Decision Engine

Orchestr8 automatically selects the best runtime based on your requirements:

| Condition              | Runtime Chosen |
| ---------------------- | -------------- |
| GPU required           | KubeVirt       |
| Large resources        | Metal3         |
| Service networking     | Kubernetes     |
| Persistence enabled    | Kubernetes     |
| Local development      | Podman         |

## Implementation Status

### ✅ Phase 1 - Core (Complete)

- [x] YAML parser with full validation
- [x] Podman adapter (build, run, stop, status, logs)
- [x] Runtime trait definition
- [x] Decision engine with auto-selection
- [x] Local state store
- [x] CLI with all commands

### ✅ Phase 2 - Kubernetes (Complete)

- [x] Generate Pod YAML from spec
- [x] Apply via kube-rs client
- [x] Service creation (ClusterIP, NodePort, LoadBalancer)
- [x] PersistentVolumeClaim handling
- [x] Health probes (liveness + readiness)
- [x] Resource limits (CPU, memory)
- [x] Complete lifecycle (run, stop, status, logs, delete)
- [x] Multi-runtime CLI support
- [x] Comprehensive documentation

### ✅ Phase 3 - TUI Dashboard (Complete)

- [x] Interactive terminal dashboard
- [x] Real-time workload status
- [x] Multi-runtime display (Podman + Kubernetes)
- [x] Integrated log viewer
- [x] Keyboard navigation (vim-style)
- [x] Auto-refresh (every 5s)
- [x] Color-coded status indicators
- [x] Runtime badges (🐳☸️)
- [x] Complete documentation

### 📅 Phase 4 - KubeVirt (Planned)

- [ ] DataVolume creation
- [ ] VirtualMachine manifest generation
- [ ] Serial console streaming

### 📅 Phase 5 - Metal3 (Planned)

- [ ] BareMetalHost discovery
- [ ] Node provisioning
- [ ] BMC integration

## Development

### Run tests

```bash
cargo test
```

### Run with verbose logging

```bash
orchestr8 -v build
```

### Deploy to Kubernetes

```bash
# Deploy to cluster
orchestr8 run --spec workload-k8s.yaml --runtime kube

# Check status
orchestr8 status web-app

# View logs
orchestr8 logs web-app --follow

# Delete
orchestr8 delete web-app
```

See `KUBERNETES.md` for complete Kubernetes guide.

### Interactive TUI

```bash
# Launch dashboard
orchestr8 tui

# Navigate with arrow keys
# Press Enter to view logs
# Press r to refresh
# Press q to quit
```

See `TUI.md` for complete TUI guide.

### Project structure

```
orchestr8/
├── src/
│   ├── main.rs          # CLI entrypoint
│   ├── lib.rs           # Library root
│   ├── spec.rs          # Workload schema
│   ├── runtime.rs       # Runtime trait
│   ├── engine.rs        # Decision engine
│   ├── state.rs         # State store
│   └── adapters/
│       ├── podman.rs    # Podman (✅ complete)
│       ├── kube.rs      # Kubernetes (✅ complete)
│       ├── kubevirt.rs  # KubeVirt stub
│       └── metal.rs     # Metal3 stub
├── workload.yaml        # Example Podman spec
├── workload-k8s.yaml    # Example Kubernetes spec
├── KUBERNETES.md        # Kubernetes guide
└── PHASE2-DELIVERABLES.md
```

## License

MIT OR Apache-2.0
