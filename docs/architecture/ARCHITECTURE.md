# 🏛️ Aether Architecture

> **Universal runtime portability** — Deploy once. Move workloads across Podman, Kubernetes, and KubeVirt without rewriting infrastructure.

See [Deployment Topologies](DEPLOYMENT-TOPOLOGIES.md) for HA (Postgres + Redis + API replicas) and [PRODUCT.md](../PRODUCT.md) for the product narrative.

---

## 📑 Table of Contents

- [System Overview](#-system-overview)
- [Core Components](#-core-components)
- [Data Flow](#-data-flow)
- [Directory Structure](#-directory-structure)
- [Key Design Decisions](#-key-design-decisions)
- [Security Architecture](#-security-architecture)

---

## 🔭 System Overview

Aether's core philosophy is **write once, deploy anywhere**. A single
workload YAML specification describes what to run, and Aether's decision
engine automatically selects the optimal runtime -- or you override
it manually. The same CLI, API, and state management layer works
identically across all three runtimes.

### High-Level Architecture

```
                          ┌─────────────────────────────────────────────┐
                          │              User Interface                 │
                          │                                             │
                          │   CLI (clap)    REST API (axum)    TUI      │
                          └────────┬──────────────┬──────────────┬──────┘
                                   │              │              │
                                   ▼              ▼              ▼
                          ┌─────────────────────────────────────────────┐
                          │             Command Router                  │
                          │         (src/commands.rs -- 40+ cmds)       │
                          └────────────────────┬────────────────────────┘
                                               │
                       ┌───────────────────────┬┴┬───────────────────────┐
                       ▼                       ▼ ▼                       ▼
              ┌────────────────┐   ┌───────────────────┐   ┌────────────────────┐
              │  Spec Parser   │   │  Decision Engine   │   │  Migration Engine  │
              │  (src/spec.rs) │   │  (src/engine.rs)   │   │ (src/migration.rs) │
              └───────┬────────┘   └────────┬──────────┘   └─────────┬──────────┘
                      │                     │                        │
                      ▼                     ▼                        ▼
              ┌─────────────────────────────────────────────────────────────┐
              │                    Runtime Trait (dyn Runtime)              │
              │            build | run | stop | status | logs | delete     │
              └──┬──────────┬──────────────┬────────────────┘
                 │          │              │
                 ▼          ▼              ▼
           ┌─────────┐ ┌──────────┐ ┌───────────┐
           │ Podman   │ │   Kube   │ │  KubeVirt │
           │ Runtime  │ │  Runtime │ │  Runtime  │
           └────┬─────┘ └────┬─────┘ └─────┬─────┘
                │            │              │
                ▼            ▼              ▼
           ┌─────────┐ ┌──────────┐ ┌───────────┐
           │ podman   │ │ kube-rs  │ │ VirtualMa-│
           │  CLI     │ │  client  │ │ chine CRD │
           └──────────┘ └──────────┘ └───────────┘

              ┌─────────────────────────────────────────────────────────┐
              │                  Cross-Cutting Concerns                 │
              │                                                         │
              │  State (JSON)  Secrets (AES-256)  Audit  Policy  Events │
              │  Metrics (Prometheus)   Health    Plugins (JSON-RPC)    │
              └─────────────────────────────────────────────────────────┘
```

---

## 🧩 Core Components

### 1. CLI Layer

| Aspect | Detail |
|---|---|
| Files | `src/main.rs`, `src/cli.rs`, `src/commands.rs` |
| Framework | [clap](https://docs.rs/clap) with derive macros |
| Subcommands | 40+ (run, build, stop, list, migrate, serve, tui, compose, secrets, drift, ...) |
| Output formats | `table` (default), `json`, `yaml`, `wide` |

**Global flags:**

| Flag | Short | Purpose |
|---|---|---|
| `--verbose` | `-v` | Enable debug-level tracing |
| `--quiet` | `-q` | Suppress all output except errors |
| `--json` | | Machine-readable JSON output |
| `--output` | `-o` | Output format (table/json/yaml/wide) |
| `--yes` | `-y` | Skip confirmation prompts (CI/automation) |
| `--dry-run` | | Show what would happen without executing |
| `--skip-policy` | | Bypass policy checks on deploy |
| `--namespace` | `-n` | Kubernetes namespace override (also `AETHER_NAMESPACE` env) |
| `--spec` | `-s` | Path to workload YAML (default: `workload.yaml`) |

---

### 2. Workload Specification

| Aspect | Detail |
|---|---|
| File | `src/spec.rs` |
| Format | YAML (`serde_yaml`) |
| API Version | `aether/v1` |
| Kind | `Workload` |

**Top-level schema fields:**

```yaml
apiVersion: aether/v1
kind: Workload
metadata:                    # name, owner, project, labels, annotations
build:                       # context, dockerfile, registry, build_args
requirements:                # cpu, memory, storage, gpu (optional)
runtime:                     # preferred (auto|container|kube|kubevirt|metal), allow list
network:                     # service flag, service_type, ports
persistence:                 # enabled, size, access_mode, storage_class
health:                      # liveness/readiness probes (httpGet, tcpSocket, exec)
config:                      # config_maps, secrets, env_from
ingress:                     # host, paths, tls, annotations
scaling:                     # min/max replicas, metrics (CPU, Memory, Custom)
```

**Validation rules:**

| Rule | Error condition |
|---|---|
| DNS-1123 names | Uppercase, underscores, dots, >63 chars, leading/trailing hyphens |
| CPU format | Empty, zero, negative, non-numeric (accepts `"2"`, `"500m"`, `"0.5"`) |
| Memory/storage format | Must have suffix: `Gi`, `Mi`, `Ti`, `Ki`, `G`, `M`, `T`, `K` |
| GPU | Count must be >0, vendor must be non-empty |
| Ports | `containerPort` and `servicePort` must be >0 |
| Scaling | `minReplicas` and `maxReplicas` must be >0, min <= max |
| Ingress | Host cannot be empty when `enabled: true` |
| Runtime preference | Preferred runtime must be in the `allow` list (unless `auto`) |

**Resource parsing (used by decision engine):**

| Input | Parsed as |
|---|---|
| `"2"` | 2.0 CPU cores |
| `"500m"` | 0.5 CPU cores (millicores) |
| `"4Gi"` | 4 GiB |
| `"512Mi"` | 512 MiB |
| `"1.5Gi"` | 1.5 GiB |

---

### 3. Decision Engine

| Aspect | Detail |
|---|---|
| File | `src/engine.rs` |
| Key type | `Engine` (stateless struct) |
| Entry point | `engine.decide(&workload) -> RuntimeKind` |

When `runtime.preferred` is set to anything other than `auto`, the engine
returns that runtime directly. When set to `auto`, it evaluates rules
in strict priority order:

```
┌──────────┐     GPU required?      ┌───────────┐
│  Start   │────── yes ────────────►│ KubeVirt  │
└──────────┘                        └───────────┘
     │ no
     ▼
  Network service?                  ┌───────────┐
  (service: true) ──── yes ────────►│   Kube    │
     │ no                           └───────────┘
     ▼
  Persistence?                      ┌───────────┐
  (enabled: true) ──── yes ────────►│   Kube    │
     │ no                           └───────────┘
     ▼
  Container allowed? ── yes ───────►┌───────────┐
     │ no                           │  Podman   │
     ▼                              └───────────┘
  Fallback: first in allow list
```

There's also a Rule 0 ahead of GPU: if `intent.compliance.isolation_required`
is set, the engine tries KubeVirt first (falling through with a warning if
KubeVirt isn't in the `allow` list).

Every rule checks the `allow` list before selecting a runtime. If a rule
matches but the target runtime is not allowed, the engine falls through to
the next rule.

---

### 4. Runtime Adapters

| Aspect | Detail |
|---|---|
| Files | `src/runtime.rs`, `src/adapters/` |
| Trait | `Runtime` (async_trait, `Send + Sync`) |

**The `Runtime` trait:**

```rust
#[async_trait]
pub trait Runtime: Send + Sync {
    async fn build(&self, spec: &Workload) -> Result<Image>;
    async fn run(&self, image: &Image, spec: &Workload) -> Result<Instance>;
    async fn stop(&self, instance: &Instance) -> Result<()>;
    async fn status(&self, instance: &Instance) -> Result<Status>;
    async fn logs(&self, instance: &Instance, follow: bool) -> Result<String>;
    async fn delete(&self, instance: &Instance) -> Result<()>;
    async fn list(&self) -> Result<Vec<Instance>>;
}
```

**Adapter details:**

| Adapter | File | Backend | Timeout | Notes |
|---|---|---|---|---|
| `PodmanRuntime` | `src/adapters/podman.rs` | Shell commands via `tokio::process::Command` | 10 min | Rootless containers, Podman health checks |
| `KubernetesRuntime` | `src/adapters/kube.rs` | `kube-rs` client | 5 min | 409 conflict handling, resource cleanup on failure |
| `KubeVirtRuntime` | `src/adapters/kubevirt.rs` | `kube-rs` + VirtualMachine CRD | 5 min | GPU passthrough, VM lifecycle |

Kube-based adapters share constructor boilerplate via the
`impl_kube_adapter_new!` macro defined in `src/adapters/mod.rs`. Each
reads `AETHER_NAMESPACE` at construction time (defaulting to `"default"`).

**Runtime factory:**

```
create_runtime(kind) ──► Box<dyn Runtime>
create_runtime_ns(kind, namespace) ──► Box<dyn Runtime>  (explicit ns)
```

---

### 5. State Management

| Aspect | Detail |
|---|---|
| File | `src/state.rs` |
| Format | JSON (`serde_json::to_string_pretty`) |
| Location | `~/.aether/state.json` |
| Crash safety | Atomic write: serialize to `.json.tmp`, then `rename()` |
| Concurrency | Advisory file locking via Unix `flock(LOCK_EX)` on `.json.lock` |

**WorkloadState fields:**

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Workload name |
| `runtime` | `RuntimeKind` | Which runtime is managing this workload |
| `instance` | `Instance` | Running instance (id, name, image, created_at) |
| `spec_path` | `PathBuf` | Path to the original YAML spec |
| `created_at` | `String` | RFC-3339 timestamp of first deployment |
| `updated_at` | `String` | RFC-3339 timestamp of last state change |

**Atomic write sequence:**

```
1. Serialize StateStore to JSON string
2. Create directory if missing
3. Open .json.lock file
4. flock(LOCK_EX) -- blocks until acquired
5. Write JSON to .json.tmp
6. rename(.json.tmp -> state.json)  -- atomic on POSIX
7. Drop lock file handle (releases flock)
```

---

### 6. Migration Engine

| Aspect | Detail |
|---|---|
| File | `src/migration.rs` |
| Key types | `MigrationPlan`, `MigrationResult`, `MigrationStrategy` |

**Three migration strategies:**

| Strategy | Downtime | Flow |
|---|---|---|
| **Immediate** | Brief | Stop source, start target, validate |
| **Blue-Green** | Zero | Start target, validate health, stop source |
| **Rolling** | Zero | Gradual traffic shift with health checks at each step |

**MigrationPlan timing parameters:**

| Parameter | Default | Purpose |
|---|---|---|
| `shutdown_delay` | 5s | Graceful shutdown wait before starting target |
| `validation_delay` | 10s | Wait before running health checks |
| `traffic_shift_interval` | 5s | Delay between rolling migration steps |
| `cleanup_delay` | 2s | Wait after stopping source before cleanup |
| `max_health_retries` | 3 | Maximum health check attempts |
| `health_retry_base_interval` | 2s | Base interval for exponential backoff |
| `rollback_on_failure` | true | Auto-rollback if target fails health checks |

**Migration flow (Blue-Green):**

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Build   │───►│  Deploy  │───►│  Health  │───►│  Switch  │───►│  Stop    │
│  target  │    │  target  │    │  check   │    │  state   │    │  source  │
└──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
                                     │                               │
                                     │ fail                          │
                                     ▼                               ▼
                               ┌──────────┐                    ┌──────────┐
                               │ Rollback │                    │  Audit   │
                               │ (delete  │                    │  log     │
                               │  target) │                    └──────────┘
                               └──────────┘
```

---

### 7. API Server

| Aspect | Detail |
|---|---|
| Files | `src/api/mod.rs`, `src/api/handlers.rs`, `src/api/types.rs` |
| Framework | [axum](https://docs.rs/axum) |
| Default port | 5090 |
| Body limit | 2 MB |
| Auth | Bearer token via `AETHER_API_KEY` env var |
| CORS | Restricted to same-origin (`http://host:port`) |

**Endpoint categories (40+ routes):**

| Category | Example routes |
|---|---|
| Workload CRUD | `GET/POST /api/workloads`, `GET/DELETE /api/workloads/:name` |
| Lifecycle | `POST .../start`, `POST .../stop`, `POST .../build`, `POST .../migrate` |
| Validation | `POST /api/validate` |
| Secrets | `GET/DELETE /api/secrets/:name`, `GET /api/secrets` |
| Monitoring | `GET /api/metrics`, `GET /api/health/:workload`, `GET /api/events` |
| Cost | `POST /api/cost` |
| Backup | `GET/POST /api/backups` |
| AI-powered | `POST /api/ai/recommend`, `GET /api/ai/profile/:name` |
| Drift/Policy | `GET /api/drift/:name`, `POST /api/policy/check` |
| Plugins | `GET /api/plugins`, `POST /api/plugins/discover` |
| Scheduler | `GET /api/scheduler/utilization`, `GET /api/scheduler/optimize` |
| Orchestrator | `GET /api/orchestrator/status`, `GET /api/orchestrator/summary` |
| Compose | `POST /api/compose/validate` |
| Dashboard | `GET /` (serves embedded HTML dashboard) |

---

### 8. Security Layer

| Aspect | Detail |
|---|---|
| Files | `src/secrets.rs`, `src/audit.rs`, `src/policy.rs` |

**Secrets management (`src/secrets.rs`):**

| Feature | Implementation |
|---|---|
| Encryption | AES-256-GCM with random nonce per operation |
| Key derivation | SHA-256 hash of raw key material |
| Fallback key | Deterministic machine-identity key (hostname + username, no process ID) |
| Storage | `~/.aether/secrets.json` |
| Namespace isolation | Secrets scoped by namespace field |
| Rotation | Configurable interval, max age, and pre-expiry notification |
| Access audit | Per-secret access log entries |

**Audit trail (`src/audit.rs`):**

| Feature | Implementation |
|---|---|
| Storage | `~/.aether/audit.json` |
| Actions tracked | Build, Deploy, Start, Stop, Delete, Migrate, Scale, ConfigChange, Backup, Restore, PolicyCheck, DriftDetected |
| Integrity | SHA-256 hash per event (`id|timestamp|action|workload|result|message`) |
| Filtering | By action type, workload name, result |

**Policy engine (`src/policy.rs`):**

| Feature | Implementation |
|---|---|
| Rule sets | `production`, `development` |
| Evaluation | Pre-deployment gate with violations and warnings |
| Severity levels | Error (blocks deploy), Warning (advisory), Info |
| Bypass | `--skip-policy` flag |
| Dry-run | `aether policy-check --policy production` |

---

### 9. Observability

| Aspect | Detail |
|---|---|
| Files | `src/metrics.rs`, `src/events.rs`, `src/health.rs` |

**Prometheus metrics (`src/metrics.rs`):**

| Metric | Type | Labels |
|---|---|---|
| `aether_workload_deployments_total` | Counter | runtime, status |
| `aether_workload_builds_total` | Counter | runtime, status |
| `aether_workload_running` | Gauge | runtime |
| `aether_migrations_total` | Counter | source_runtime, target_runtime, strategy, status |
| `aether_migration_duration_seconds` | Histogram | source_runtime, target_runtime, strategy |
| `aether_cli_commands_total` | Counter | command |
| `aether_command_duration_seconds` | Histogram | command |
| `aether_runtime_available` | Gauge | runtime |

**Event system (`src/events.rs`):**

| Feature | Detail |
|---|---|
| Severity levels | Info, Warning, Error, Critical |
| Filtering | By workload, severity, time range |
| Summary | Aggregated event counts and timeline |

**Health monitoring (`src/health.rs`):**

| Feature | Detail |
|---|---|
| Per-workload history | Timestamped health check results |
| Uptime calculation | Based on health check success ratio |
| Restart tracking | Count and timestamps of restarts |
| Background loop | Configurable interval with circuit-breaker protection |

---

### 10. Plugin System

| Aspect | Detail |
|---|---|
| File | `src/plugin.rs` |
| Protocol | JSON-RPC style over stdin/stdout (one message per line) |
| Discovery | Scan `~/.aether/plugins/` for `*.json` manifest files |
| Registry | Persistent `~/.aether/plugins.json` |
| Timeout | 60 seconds per IPC call |

**Manifest format:**

```json
{
  "name": "my-runtime",
  "version": "1.0.0",
  "runtime_kind": "custom-wasm",
  "command": "/usr/local/bin/my-runtime",
  "capabilities": ["build", "run", "stop", "status", "delete", "list"]
}
```

**Plugin IPC protocol:**

```
aether ──stdin──►  {"type":"BuildRequest","spec_json":"{...}"}
plugin ──stdout──► {"type":"BuildResponse","image_json":"{...}"}

aether ──stdin──►  {"type":"RunRequest","image_json":"{...}","spec_json":"{...}"}
plugin ──stdout──► {"type":"RunResponse","instance_json":"{...}"}
```

Plugins participate in the same lifecycle as built-in runtimes via the
`PluginRuntime` struct, which implements the `Runtime` trait. Capability
checking happens before each operation -- calling `build` on a plugin that
only supports `["run", "stop"]` returns a descriptive error.

---

## 🔄 Data Flow

### Workload Deployment Flow

```
                  workload.yaml
                       │
                       ▼
              ┌────────────────┐
              │  Parse & Load  │    Workload::from_file()
              │   (serde_yaml) │    Validates apiVersion, kind, DNS names,
              └───────┬────────┘    resources, ports, scaling, ingress
                      │
                      ▼
              ┌────────────────┐
              │  Policy Check  │    PolicyEngine::evaluate()
              │  (optional)    │    production / development rule sets
              └───────┬────────┘    --skip-policy to bypass
                      │
                      ▼
              ┌────────────────┐
              │ Engine Decide  │    Engine::decide()
              │                │    GPU → KubeVirt,
              └───────┬────────┘    Service → Kube, Default → Podman
                      │
                      ▼
              ┌────────────────┐
              │  Build Image   │    runtime.build(&spec) -> Image
              └───────┬────────┘    Podman: podman build, Kube: N/A
                      │
                      ▼
              ┌────────────────┐
              │    Deploy      │    runtime.run(&image, &spec) -> Instance
              └───────┬────────┘    Creates containers/pods/VMs/BMHs
                      │
                      ▼
              ┌────────────────┐
              │  State Update  │    StateStore::upsert() + atomic save
              └───────┬────────┘    ~/.aether/state.json
                      │
                      ▼
              ┌────────────────┐
              │   Audit Log    │    AuditTrail::record_deploy()
              └───────┬────────┘    SHA-256 integrity hash
                      │
                      ▼
              ┌────────────────┐
              │ Metrics Update │    Increment deployment counters
              └────────────────┘
```

### Migration Flow (Blue-Green)

```
  ┌─────────────┐
  │ Load state  │    StateStore::get(workload_name)
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │ Build on    │    target_runtime.build(&spec)
  │ target      │
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │ Deploy on   │    target_runtime.run(&image, &spec)
  │ target      │
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐     fail     ┌──────────────┐
  │ Health      │─────────────►│  Rollback:   │
  │ check with  │              │  delete      │
  │ exp backoff │              │  target,     │
  └──────┬──────┘              │  keep source │
         │ pass                └──────────────┘
         ▼
  ┌─────────────┐
  │ Stop source │    source_runtime.stop(&instance)
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐
  │ Update      │    StateStore: swap runtime + instance
  │ state       │    AuditTrail: record migration
  └─────────────┘
```

### API Request Flow

```
  HTTP Request
       │
       ▼
  ┌──────────────┐
  │   Auth       │    Bearer token check (AETHER_API_KEY)
  │  Middleware   │    Public: / and /health
  └──────┬───────┘    Rejects with 401 if invalid
         │
         ▼
  ┌──────────────┐
  │    CORS      │    Restricted to same-origin
  │   Layer      │    Methods: GET, POST, DELETE
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │  Body Limit  │    Max 2 MB per request
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │   Handler    │    Route-specific logic
  │              │    Reads/writes AppState (Arc<RwLock<StateStore>>)
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │   JSON       │    { success: bool, data: T, error: Option<String> }
  │  Response    │
  └──────────────┘
```

---

## 📁 Directory Structure

```
src/
├── main.rs                  # Entry point, tokio runtime setup
├── cli.rs                   # Clap argument definitions, global flags
├── commands.rs              # 40+ subcommand implementations
├── lib.rs                   # Library root, re-exports, error types
├── spec.rs                  # Workload YAML schema and validation
├── engine.rs                # Rule-based runtime decision engine
├── runtime.rs               # Runtime trait definition and factory
├── migration.rs             # Migration strategies (immediate/blue-green/rolling)
├── state.rs                 # JSON state store with atomic write + flock
├── config.rs                # Application configuration
├── output.rs                # Output formatting (table, JSON, YAML, wide)
├── completions.rs           # Shell completion generation (bash/zsh/fish)
├── resources.rs             # Shared helpers: timestamps, path utils, parsing
├── secrets.rs               # AES-256-GCM encrypted secrets management
├── audit.rs                 # Audit trail with SHA-256 integrity hashes
├── policy.rs                # Pre-deployment policy engine
├── metrics.rs               # Prometheus metrics export
├── events.rs                # Event tracking with severity levels
├── health.rs                # Health history, uptime, restart tracking
├── drift.rs                 # Drift detection and reconciliation
├── backup.rs                # State backup and restore
├── cost.rs                  # Multi-cloud cost estimation
├── templates.rs             # Built-in workload spec templates
├── compose.rs               # Multi-workload compose file support
├── plugin.rs                # Plugin system (JSON-RPC IPC)
├── dependencies.rs          # Workload dependency graph
├── environments.rs          # Environment management (dev/staging/prod)
├── sla.rs                   # SLA compliance monitoring
├── scheduler.rs             # Resource utilization and optimization
├── orchestrator.rs          # Multi-workload orchestration and health loops
├── adapters/
│   ├── mod.rs               # Adapter module, impl_kube_adapter_new! macro
│   ├── common.rs            # Shared adapter utilities
│   ├── podman.rs            # PodmanRuntime: shell-based container management
│   ├── kube.rs              # KubernetesRuntime: kube-rs Deployment/Service/PVC
│   └── kubevirt.rs          # KubeVirtRuntime: VirtualMachine CRD management
├── api/
│   ├── mod.rs               # Axum router, auth middleware, CORS, server start
│   ├── handlers.rs          # 40+ route handler implementations
│   └── types.rs             # ApiConfig, AppState, request/response types
├── ai/
│   ├── mod.rs               # AI module root
│   ├── analyzer.rs          # Log analysis and anomaly detection
│   ├── scoring.rs           # Workload scoring for runtime selection
│   ├── profiler.rs          # Workload profiling and recommendations
│   ├── migration.rs         # AI-assisted migration advice
│   ├── scaling.rs           # Scaling recommendations
│   └── affinity.rs          # Runtime affinity classification
└── ui/
    ├── mod.rs               # TUI module root
    ├── app.rs               # TUI application state and event loop
    ├── dashboard.rs         # Dashboard view (workload table, status)
    ├── components.rs        # Reusable TUI widgets
    ├── events.rs            # TUI event handling (keyboard, tick)
    └── logs.rs              # Log streaming view
```

---

## 🧭 Key Design Decisions

### Why Rust?

| Reason | Benefit |
|---|---|
| Memory safety without GC | No null pointer dereferences, no data races at compile time |
| Async ecosystem (tokio) | Efficient concurrent I/O for kube-rs, Podman shell commands, API server |
| Single static binary | Zero runtime dependencies, easy distribution |
| `serde` ecosystem | Seamless serialization between YAML specs, JSON state, and Kubernetes API objects |
| Strong type system | Spec validation errors caught at compile time via typed enums |

### Why Trait-Based Adapters?

The `Runtime` trait enables **open/closed extension**: new runtimes can be
added without modifying existing adapters. The plugin system takes this
further -- any external binary that speaks JSON-RPC can act as a runtime,
discovered at runtime from `~/.aether/plugins/`.

```
Built-in: PodmanRuntime, KubernetesRuntime, KubeVirtRuntime
Plugin:   PluginRuntime (wraps any binary that speaks JSON-RPC)
```

### Why Local JSON State vs Distributed State?

Aether is designed as a **single-node CLI + API server**, not a distributed
control plane. This simplifies the architecture considerably:

| Choice | Rationale |
|---|---|
| JSON file | Human-readable, debuggable with `cat`, no database dependency |
| `~/.aether/` directory | Follows XDG conventions, user-scoped, no root required |
| Single-writer model | Advisory flock prevents concurrent corruption |
| No etcd/consul | Kubernetes already has etcd; Aether manages the _intent_, not the _cluster state_ |

For multi-user or HA scenarios, the API server provides shared access to
a single state file via `Arc<RwLock<StateStore>>`.

### Why Atomic Writes + File Locking?

State must survive crashes without corruption. Two mechanisms work together:

1. **Atomic rename:** Write to `.json.tmp` then `rename()` to `state.json`.
   On POSIX systems, `rename()` is atomic -- readers see either the old
   file or the new file, never a partial write.

2. **Advisory flock:** `flock(LOCK_EX)` on a `.json.lock` file prevents
   concurrent `aether` processes from writing simultaneously. The lock is
   blocking (`LOCK_EX` without `LOCK_NB`), so concurrent processes wait
   instead of failing.

Together these provide crash safety without requiring a database engine.

### Why SHA-256 for Key Derivation?

The secrets engine derives an AES-256 key by hashing raw key material with
SHA-256. This is deterministic (same input always produces the same key),
which is necessary because the key must be reproducible from the same
machine-identity inputs without storing a salt. The 256-bit SHA-256 output
maps directly to the 256-bit AES key size.

---

## 🔒 Security Architecture

### API Authentication Flow

```
Client                              API Server
  │                                      │
  │  GET /api/workloads                  │
  │  Authorization: Bearer <token>       │
  │ ────────────────────────────────────► │
  │                                      │
  │            ┌─────────────────────┐   │
  │            │ auth_middleware()   │   │
  │            │                     │   │
  │            │ 1. Read AETHER_API_ │   │
  │            │    KEY from env     │   │
  │            │ 2. If unset/empty:  │   │
  │            │    allow (local dev)│   │
  │            │ 3. If /health or /: │   │
  │            │    allow (public)   │   │
  │            │ 4. Compare Bearer   │   │
  │            │    token to key     │   │
  │            │ 5. Match: pass      │   │
  │            │    Mismatch: 401    │   │
  │            └─────────────────────┘   │
  │                                      │
  │  200 OK  { success: true, data: [] } │
  │ ◄──────────────────────────────────── │
```

### Encryption at Rest

Secrets are encrypted using AES-256-GCM with a **random 96-bit nonce per
operation**. The encryption key is derived as follows:

```
Key source (in priority order):
  1. AETHER_SECRET_KEY environment variable
  2. Machine-identity fallback: SHA-256(hostname + username)
     (Deterministic -- no process ID, so the key survives restarts)

Key derivation:
  raw_key_material ──► SHA-256 ──► 256-bit AES key

Encryption:
  plaintext + random_nonce ──► AES-256-GCM ──► base64(nonce || ciphertext)

Decryption:
  base64_decode ──► split(nonce, ciphertext) ──► AES-256-GCM ──► plaintext
```

### Audit Integrity

Each audit event includes a SHA-256 hash computed over its core fields:

```
hash = SHA-256( id | timestamp | action | workload | result | message )
```

This allows tamper detection: if any field is modified after recording, the
integrity hash will not match a recomputation. The hash is stored in the
`integrity_hash` field of each `AuditEvent`.

### File Permissions

| File | Permission | Rationale |
|---|---|---|
| Backup archives | `0o600` | Contains full state, potentially sensitive |
| Snapshot files | `0o600` | Point-in-time state, user-only access |
| Secrets store | `0o600` | Contains encrypted secret values |
| State file | Default | Non-sensitive metadata (names, runtimes, timestamps) |

### Input Validation Summary

| Layer | Validation |
|---|---|
| **Spec parsing** | DNS-1123 names, resource format, port ranges, scaling bounds |
| **Path handling** | Traversal prevention (no `../` in spec paths) |
| **API body** | 2 MB request body limit |
| **Kubernetes** | Resource cleanup on failure (delete orphaned Deployments/Services/PVCs) |
| **Runtime names** | Validated against known set + plugin registry |

---

## 🔗 Cross-References

| Document | Relevance |
|---|---|
| [Workload Schema](../reference/SCHEMA.md) | Complete YAML specification reference |
| [API Reference](../reference/api/API-Reference.md) | Full REST API documentation |
| [Plugin System](../features/plugins.md) | Plugin development guide |
| [Security Guide](../features/security.md) | Security best practices |
| [Metrics & Monitoring](../guides/operations/METRICS.md) | Prometheus integration |
| [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md) | Operational migration procedures |
| [Backup & Restore](../guides/operations/BACKUP.md) | State backup and disaster recovery |
| [CLI Reference](../guides/cli/CLI-Reference.md) | Complete command documentation |

---

*Architecture documentation for Aether v0.3.0.*
