# 🔬 Tutorial 4: How Aether Works -- A Deep Dive

> **Estimated time:** 90 minutes
> **Level:** Advanced / Contributor
> **Prerequisites:** [Tutorial 1 - Beginner Deployment](./01-beginner-deployment.md), [Tutorial 2 - Intermediate Workflows](./02-intermediate-workflows.md)
> **License:** Proprietary HyperSDK

---

## 📑 Table of Contents

- [Part 1: The Workload Specification (15 min)](#-part-1-the-workload-specification)
- [Part 2: The Decision Engine (15 min)](#-part-2-the-decision-engine)
- [Part 3: Runtime Adapters (20 min)](#-part-3-runtime-adapters)
- [Part 4: State Management (10 min)](#-part-4-state-management)
- [Part 5: The Migration Engine (15 min)](#-part-5-the-migration-engine)
- [Part 6: Security Model (10 min)](#-part-6-security-model)
- [Part 7: The API Server (5 min)](#-part-7-the-api-server)
- [Part 8: Observability & Events (5 min)](#-part-8-observability--events)
- [Summary](#-summary)

---

## What you'll learn

This tutorial takes you through Aether's internals -- the Rust code, the
algorithms, and the design decisions. By the end you will understand:

- How a `workload.yaml` is parsed, validated, and mapped to Rust structs
- How the decision engine selects a runtime
- How each runtime adapter translates the spec into infrastructure
- How state is persisted crash-safely with atomic writes and file locking
- How the migration engine orchestrates zero-downtime moves between runtimes
- How secrets are encrypted with AES-256-GCM
- How the API server is structured and authenticated
- How events, metrics, and audit logs form the observability layer

---

## 📦 Part 1: The Workload Specification

*Estimated time: 15 minutes*

Every Aether deployment starts with a **workload spec** -- a YAML file that
describes what to deploy and how. This section walks through every field,
shows the backing Rust structs, and explains the validation rules.

### 1.1 Full annotated workload.yaml

```yaml
# --- Identity ---------------------------------------------------------------
apiVersion: aether/v1          # Must be exactly "aether/v1"
kind: Workload                 # Must be exactly "Workload"

# --- Metadata ---------------------------------------------------------------
metadata:
  name: api-service            # DNS-1123 label, 1-63 chars, [a-z0-9-]
  owner: platform-team         # Team or individual owner
  project: backend             # Logical project grouping
  labels:                      # Arbitrary key-value pairs (filtering/selection)
    app: api-service
    tier: backend
  annotations:                 # Non-identifying metadata
    description: "Core REST API"

# --- Build ------------------------------------------------------------------
build:
  context: .                   # Docker build context directory
  dockerfile: Dockerfile       # Path to Dockerfile (relative to context)
  registry: ghcr.io/my-org     # Image registry prefix
  buildArgs:                   # Build-time arguments passed to docker/podman
    RUST_VERSION: "1.78"

# --- Resource requirements --------------------------------------------------
requirements:
  cpu: "4"                     # "4" = 4 cores, "500m" = 0.5 cores
  memory: 8Gi                  # Kubernetes-style: Gi, Mi, Ki, Ti, G, M, K, T
  storage: 50Gi                # Same suffixes as memory
  gpu:                         # Optional GPU requirements
    count: 1
    vendor: nvidia             # "nvidia", "amd", or "intel"

# --- Runtime selection ------------------------------------------------------
runtime:
  preferred: auto              # auto | container | kube | kubevirt | metal
  allow:                       # Which runtimes the engine may consider
    - container
    - kube

# --- Networking -------------------------------------------------------------
network:
  service: true                # Create a Kubernetes Service?
  serviceType: ClusterIP       # ClusterIP | NodePort | LoadBalancer
  ports:
    - containerPort: 8080      # Port inside the container
      servicePort: 80          # Exposed service port
      protocol: TCP            # TCP or UDP

# --- Persistence ------------------------------------------------------------
persistence:
  enabled: true
  size: 100Gi
  accessMode: ReadWriteOnce    # ReadWriteOnce | ReadOnlyMany | ReadWriteMany
  storageClass: fast-ssd       # Optional Kubernetes StorageClass name

# --- Health probes ----------------------------------------------------------
health:
  liveness:
    path: /healthz
    port: 8080
    initialDelaySeconds: 10
    periodSeconds: 30
  readiness:
    path: /ready
    port: 8080
    initialDelaySeconds: 5
    periodSeconds: 10
```

### 1.2 Backing Rust structs

The workload YAML maps directly to the `Workload` struct in `src/spec.rs`.
The `#[serde(rename_all = "camelCase")]` attribute handles the YAML camelCase
to Rust snake_case conversion automatically.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workload {
    pub api_version: String,        // apiVersion in YAML
    pub kind: String,
    pub metadata: Metadata,
    pub build: BuildSpec,
    pub requirements: ResourceRequirements,
    pub runtime: RuntimeSpec,
    pub network: NetworkSpec,        // #[serde(default)]
    pub persistence: PersistenceSpec,// #[serde(default)]
    pub health: Option<HealthSpec>,  // #[serde(default)]
    pub config: Option<ConfigSpec>,
    pub ingress: Option<IngressSpec>,
    pub scaling: Option<ScalingSpec>,
}
```

Key sub-structs:

| Struct                 | YAML section     | Key fields                                          |
|------------------------|------------------|-----------------------------------------------------|
| `Metadata`             | `metadata`       | name, owner, project, labels, annotations           |
| `BuildSpec`            | `build`          | context, dockerfile, registry, build_args           |
| `ResourceRequirements` | `requirements`   | cpu, memory, storage, gpu (optional)                |
| `RuntimeSpec`          | `runtime`        | preferred (enum), allow (Vec of enum)               |
| `NetworkSpec`          | `network`        | service (bool), service_type, ports                 |
| `PersistenceSpec`      | `persistence`    | enabled, size, access_mode, storage_class           |
| `HealthSpec`           | `health`         | liveness (optional), readiness (optional)           |

### 1.3 Validation rules

When `Workload::from_file()` is called, it deserializes the YAML and then
calls `validate()`. Here are the rules enforced:

| Rule                                | Check                                                     |
|-------------------------------------|-----------------------------------------------------------|
| API version                         | Must be exactly `"aether/v1"`                             |
| Kind                                | Must be exactly `"Workload"`                              |
| Name format                         | DNS-1123 label: `[a-z0-9-]`, 1--63 chars, no leading/trailing hyphen |
| CPU format                          | Whole number (`"4"`), fractional (`"0.5"`), or millicores (`"500m"`) |
| Memory / Storage format             | Must have a suffix: `Gi`, `Mi`, `Ki`, `Ti`, `G`, `M`, `K`, `T` |
| GPU count                           | Must be > 0 if GPU section is present                     |
| GPU vendor                          | Must be non-empty if GPU section is present               |
| Port values                         | containerPort and servicePort must be > 0                 |
| Scaling                             | minReplicas > 0, maxReplicas > 0, min <= max              |
| Ingress host                        | Must be non-empty when ingress.enabled is true            |
| Preferred runtime in allow list     | Unless `auto`, preferred must appear in the allow list    |

### 1.4 Resource parsing

CPU and memory strings are parsed into numeric values for the decision engine:

```
CPU: "2"       -> 2.0 cores
CPU: "500m"    -> 0.5 cores    (millicores / 1000)
CPU: "0.5"     -> 0.5 cores

Memory: "4Gi"     -> 4 * 1024^3 bytes
Memory: "4096Mi"  -> 4096 * 1024^2 bytes
Memory: "512Ki"   -> 512 * 1024 bytes
Memory: "1.5Gi"   -> 1.5 * 1024^3 bytes
```

> 💡 **Note:** Invalid or unparseable CPU values default to 1.0 core,
> and invalid memory values default to 1Gi for runtime decision purposes.
> Validation catches these before deployment, so the defaults only apply
> to the decision engine's internal calculations.

---

## 🧠 Part 2: The Decision Engine

*Estimated time: 15 minutes*

When `runtime.preferred` is set to `auto`, Aether's decision engine
(`src/engine.rs`) evaluates a priority-ordered rule chain to pick the
best runtime for your workload.

### 2.1 The rule chain

```
┌──────────────────────────────────────────────────────────┐
│                   Workload Spec Parsed                   │
└──────────────┬───────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────────┐
│  Rule 0: Isolation required?                              │
│  intent.compliance.isolation_required == true             │
│  && kubevirt in allow list                                │
│  YES ──────────────────────────────────── ► KubeVirt     │
└──────────────┬───────────────────────────────────────────┘
               │ NO (or KubeVirt not allowed — falls through with a warning)
               ▼
┌──────────────────────────────────────────────────────────┐
│  Rule 1: GPU required?                                   │
│  gpu.is_some() && kubevirt in allow list                 │
│  YES ──────────────────────────────────── ► KubeVirt     │
└──────────────┬───────────────────────────────────────────┘
               │ NO
               ▼
┌──────────────────────────────────────────────────────────┐
│  Rule 2: Network service enabled?                        │
│  network.service == true && kube in allow list           │
│  YES ──────────────────────────────────── ► Kubernetes   │
└──────────────┬───────────────────────────────────────────┘
               │ NO
               ▼
┌──────────────────────────────────────────────────────────┐
│  Rule 3: Persistence enabled?                             │
│  persistence.enabled == true && kube in allow list        │
│  YES ──────────────────────────────────── ► Kubernetes   │
└──────────────┬───────────────────────────────────────────┘
               │ NO
               ▼
┌──────────────────────────────────────────────────────────┐
│  Default: container in allow list?                       │
│  YES ──────────────────────────────────── ► Podman       │
└──────────────┬───────────────────────────────────────────┘
               │ NO
               ▼
┌──────────────────────────────────────────────────────────┐
│  Fallback: pick first item from allow list               │
│  Empty allow list ──────────────────────── ► ERROR       │
└──────────────────────────────────────────────────────────┘
```

### 2.2 Allow-list filtering

Every rule checks `is_allowed()` before selecting a runtime. If the
target runtime is not in the spec's `runtime.allow` list, the rule is
skipped and the engine falls through to the next rule.

This means you can have GPU requirements but still force Podman:

```yaml
runtime:
  preferred: auto
  allow:
    - container          # Only container is allowed
```

Even though GPU triggers Rule 1, KubeVirt is not allowed, so the engine
skips to the default and picks Podman.

### 2.3 Example: same workload, different runtimes

Consider this base spec:

```yaml
requirements:
  cpu: "2"
  memory: 4Gi
runtime:
  preferred: auto
  allow: [container, kube, kubevirt]
```

| Change                         | Engine decision   | Rule triggered                |
|--------------------------------|-------------------|-------------------------------|
| *(no change)*                  | Podman            | Default (no triggers)         |
| `gpu: { count: 1, vendor: nvidia }` | KubeVirt    | Rule 1: GPU                   |
| `network.service: true`       | Kubernetes        | Rule 2: Network service       |
| `persistence.enabled: true`   | Kubernetes        | Rule 3: Persistence           |

### 2.4 Rule priority

When multiple rules match simultaneously, the **highest priority rule wins**:

```
Isolation (Rule 0) > GPU (Rule 1) > Network (Rule 2) > Persistence (Rule 3) > Default
```

A workload with GPU requirements, network service, and persistence will
always choose KubeVirt (Rule 1), regardless of the other properties.

---

## 🔌 Part 3: Runtime Adapters

*Estimated time: 20 minutes*

Every runtime adapter implements the `Runtime` trait defined in
`src/runtime.rs`. This is the universal interface that makes Aether
runtime-agnostic.

### 3.1 The Runtime trait

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

Each method returns an `Instance`, `Image`, or `Status` struct -- common
types that abstract over runtime-specific details:

```rust
pub struct Image {
    pub name: String,
    pub tag: String,
    pub digest: Option<String>,
    pub runtime: RuntimeKind,      // Podman | Kubernetes | KubeVirt
}

pub struct Instance {
    pub id: String,                // Container ID, Pod name, VM name, etc.
    pub name: String,
    pub runtime: RuntimeKind,
    pub image: String,
    pub created_at: String,        // RFC 3339 timestamp
}
```

### 3.2 Podman adapter

**Location:** `src/adapters/podman.rs`

The Podman adapter shells out to the `podman` CLI using `tokio::process::Command`.

```
aether run
  │
  ▼
PodmanRuntime::build()
  │  podman build -t <registry>/<name>:latest -f <dockerfile> <context>
  │  --build-arg KEY=VALUE ...
  ▼
PodmanRuntime::run()
  │  podman run -d --name <name> -p <host>:<container> <image>
  │  (+ health check flags if health spec is present)
  ▼
Instance { id: <container-id>, runtime: Podman, ... }
```

Key implementation details:

| Feature              | Detail                                                  |
|----------------------|---------------------------------------------------------|
| **Timeout**          | 10-minute timeout on all podman commands (`PODMAN_TIMEOUT`) |
| **Error handling**   | Non-zero exit code -> bail with stderr message          |
| **Availability**     | Checks for `podman` binary via `which::which()` at construction |
| **Health checks**    | Maps Aether health probes to Podman native `--health-cmd` flags |
| **Port mapping**     | `-p <servicePort>:<containerPort>` for each port entry  |

### 3.3 Kubernetes adapter

**Location:** `src/adapters/kube.rs`

The Kubernetes adapter uses the `kube-rs` client library to create native
Kubernetes resources.

```
aether run (runtime = kube)
  │
  ▼
KubernetesRuntime::build()
  │  (no-op or podman build; image assumed pre-built for kube)
  ▼
KubernetesRuntime::run()
  │
  ├─ Create ConfigMap  (if config.config_maps is present)
  ├─ Create Secret     (if config.secrets is present)
  ├─ Create PVC        (if persistence.enabled)
  ├─ Create Pod        (with resource limits, probes, env)
  ├─ Create Service    (if network.service)
  ├─ Create Ingress    (if ingress.enabled)
  └─ Create HPA        (if scaling.enabled)
  │
  ▼
Instance { id: <pod-name>, runtime: Kubernetes, ... }
```

Key implementation details:

| Feature                | Detail                                                      |
|------------------------|-------------------------------------------------------------|
| **Client**             | `kube::Client` from in-cluster config or kubeconfig         |
| **Namespace**          | `AETHER_NAMESPACE` env, `--namespace` flag, or `"default"`  |
| **Conflict handling**  | HTTP 409 on create -> resource already exists (logged)      |
| **Cleanup on failure** | If Pod creation fails, previously created ConfigMap/Secret/PVC/Service/Ingress are deleted |
| **Resource mapping**   | CPU/Memory strings passed directly as Kubernetes `Quantity` |
| **Health probes**      | Mapped to Kubernetes `livenessProbe` / `readinessProbe`     |

### 3.4 KubeVirt adapter

**Location:** `src/adapters/kubevirt.rs`

The KubeVirt adapter creates virtual machines via the KubeVirt CRD API.

```
aether run (runtime = kubevirt)
  │
  ▼
KubeVirtRuntime::build()
  │  (builds container image for the DataVolume source)
  ▼
KubeVirtRuntime::run()
  │
  ├─ Create DataVolume    (CRD: cdi.kubevirt.io/v1beta1)
  │    - source: registry image
  │    - storage: from requirements.storage
  └─ Create VirtualMachine (CRD: kubevirt.io/v1)
       - CPU cores, memory from requirements
       - Disk from DataVolume
       - GPU passthrough if gpu spec present
  │
  ▼
Instance { id: <vm-name>, runtime: KubeVirt, ... }
```

Both CRDs are accessed via `kube::core::DynamicObject` and the
`discover_crd_api()` helper that resolves the API resource at runtime.

### 3.5 Namespace override

All kube-based adapters (`Kubernetes`, `KubeVirt`) support
namespace override through two mechanisms, in priority order:

1. **CLI flag:** `aether run --namespace staging`
2. **Environment variable:** `AETHER_NAMESPACE=staging`
3. **Default:** Each adapter has a built-in default (`"default"` for
   Kubernetes/KubeVirt)

Internally, the `create_runtime_ns()` factory function passes the namespace
to the adapter's `with_namespace()` constructor.

---

## 💾 Part 4: State Management

*Estimated time: 10 minutes*

Aether stores all runtime state on the local filesystem under `~/.aether/`.
There is no external database -- this keeps the tool self-contained and
zero-dependency.

### 4.1 Directory layout

```
~/.aether/
├── state.json          # Active workload instances
├── state.json.lock     # Advisory lock file (flock)
├── state.json.tmp      # Temporary file during atomic writes
├── secrets.json        # Encrypted secrets store
├── audit.json          # Audit trail with integrity hashes
├── config.yaml         # User configuration
├── backups/            # State backups (0o600 permissions)
│   ├── backup-20260412T103000Z.json
│   └── ...
├── snapshots/          # Point-in-time snapshots (0o600 permissions)
│   └── ...
└── plugins/            # Plugin manifests (*.json)
    └── wasm-runtime.json
```

### 4.2 The StateStore struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateStore {
    pub workloads: HashMap<String, WorkloadState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadState {
    pub name: String,
    pub runtime: RuntimeKind,
    pub instance: Instance,       // { id, name, runtime, image, created_at }
    pub spec_path: PathBuf,       // Path to the original workload.yaml
    pub created_at: String,       // RFC 3339
    pub updated_at: String,       // RFC 3339
}
```

The state is a simple `HashMap<String, WorkloadState>` keyed by workload
name. Operations:

| Method     | Description                                     |
|------------|-------------------------------------------------|
| `upsert()` | Insert or replace a workload entry              |
| `get()`    | Retrieve by name (returns `Option`)             |
| `remove()` | Delete and return the entry                     |
| `list()`   | Return all workload states                      |

### 4.3 Atomic write pattern

State writes use a three-step process to prevent data loss on crash:

```
Step 1: Serialize to JSON string
            │
Step 2: Write to ~/.aether/state.json.tmp
            │
Step 3: Rename state.json.tmp → state.json  (atomic on POSIX)
```

The `rename()` syscall is atomic on POSIX filesystems. This means the
state file is either fully old or fully new -- never a partial write.

```rust
// From StateStore::save()
let tmp_path = path.with_extension("json.tmp");
std::fs::write(&tmp_path, &content)?;       // Step 2
std::fs::rename(&tmp_path, path)?;          // Step 3 (atomic)
```

### 4.4 Advisory file locking

To prevent two concurrent `aether` processes from corrupting state,
`save()` acquires an exclusive advisory lock using `flock(LOCK_EX)`:

```
Process A: aether run web-app
  │  flock(LOCK_EX) on state.json.lock  ← blocks until acquired
  │  write state.json.tmp
  │  rename → state.json
  │  drop lock_file → flock released
  │
Process B: aether run api-service      (started concurrently)
  │  flock(LOCK_EX) on state.json.lock  ← waits for Process A
  │  (proceeds after A releases lock)
```

On non-Unix platforms, the lock is a no-op (documented limitation).

### 4.5 Loading state

`StateStore::load()` returns an empty store if the file does not exist.
This means fresh installations work out of the box without initialization.

---

## 🔄 Part 5: The Migration Engine

*Estimated time: 15 minutes*

The migration engine (`src/migration.rs`) moves workloads between runtimes
with three strategies, health validation, and automatic rollback.

### 5.1 Migration strategies

#### Immediate

The simplest strategy. Has a brief downtime window while the source is
stopped and the target starts.

```
Source (Podman)                 Target (Kubernetes)
────────────────────            ────────────────────
   [RUNNING]
        │
   stop source
   wait shutdown_delay (5s)
        │                       build image
        │                       run on target
        │                       wait validation_delay (10s)
        │                       health check
        │                           │
        │                    ┌──────┴──────┐
        │                    │ HEALTHY?    │
        │                    ├─ YES ───────┤
   delete source             │  update     │
                             │  state      │
                             └─────────────┘
                             │ NO (rollback)│
                        ┌────┤  enabled?    │
                        │    └─────────────┘
                   rebuild source
                   restart source
                   report failure
```

#### Blue-Green

Zero-downtime. The new (green) deployment runs alongside the old (blue)
deployment. Traffic switches only after the green deployment passes health
checks.

```
Source (Blue)                    Target (Green)
────────────────────            ────────────────────
   [RUNNING]                    build image
   [RUNNING]                    run on target
   [RUNNING]                    wait validation_delay
   [RUNNING]                    health check ──── FAIL? → delete green
        │                            │                    keep blue running
        │                       [HEALTHY]
        │                            │
   switch traffic ◄──────────────────┘
   wait cleanup_delay (2s)
   stop blue
   delete blue
                                [RUNNING - sole instance]
                                update state
```

#### Rolling

The most sophisticated strategy. Uses exponential backoff for health
checks and gradual traffic shifting at 25% increments.

```
Source                          Target
────────────────────            ────────────────────
   [RUNNING]                    build image
   [RUNNING]                    run on target
   [RUNNING]                    wait validation_delay
   [RUNNING]                    health check (with retries)
        │                            │
        │                       retry schedule (exponential backoff):
        │                         attempt 1: wait base (2s)
        │                         attempt 2: wait base*2 (4s)
        │                         attempt 3: wait base*4 (8s)
        │                         (capped at 30s per wait)
        │                            │
        │                       [HEALTHY]
        │                            │
   ┌────┴────────────────────────────┘
   │ Gradual traffic shift:
   │   25% → target  (health check)
   │   50% → target  (health check)
   │   75% → target  (health check)
   │  100% → target  (health check)
   │        interval: traffic_shift_interval (5s)
   │
   │ If target becomes unhealthy during shift:
   │   → delete target, keep source, report failure
   │
   stop source
   wait shutdown_delay (5s)
   delete source
                                [RUNNING - sole instance]
                                update state
```

### 5.2 Health validation with exponential backoff

The rolling strategy retries health checks with exponential backoff:

```
backoff_duration = base_interval * 2^(attempt - 1)
capped_duration  = min(backoff_duration, 30 seconds)
```

With default settings (`base_interval = 2s`, `max_health_retries = 3`):

| Attempt | Wait before retry | Cumulative time |
|---------|-------------------|-----------------|
| 1       | 2s                | 2s              |
| 2       | 4s                | 6s              |
| 3       | 8s                | 14s             |

If all retries are exhausted and the target is still unhealthy, the
target instance is deleted and the source keeps running.

### 5.3 MigrationPlan defaults

```rust
pub struct MigrationPlan {
    pub validation_delay: Duration,          // 10s
    pub shutdown_delay: Duration,            // 5s
    pub traffic_shift_interval: Duration,    // 5s
    pub cleanup_delay: Duration,             // 2s
    pub max_health_retries: u32,             // 3
    pub health_retry_base_interval: Duration,// 2s
    pub rollback_on_failure: bool,           // configurable
}
```

### 5.4 Automatic rollback

When `rollback_on_failure` is `true` and the immediate strategy detects
a failure (deployment error or health check failure):

1. Delete the failed target instance (if it was created)
2. Rebuild the image on the source runtime
3. Redeploy on the source runtime
4. Return `MigrationResult { success: false, rollback_performed: true }`

The blue-green and rolling strategies do not rebuild the source -- they
simply keep the source running since it was never stopped.

### 5.5 Migration advice AI

Before migrating, you can get an AI recommendation:

```bash
aether migration-advice my-app kube
```

This analyzes workload characteristics (resource requirements, health
probes, persistence needs) and recommends the best migration strategy
with a risk assessment and estimated migration time.

---

## 🔐 Part 6: Security Model

*Estimated time: 10 minutes*

### 6.1 Secrets encryption (AES-256-GCM)

All secrets are encrypted at rest using AES-256-GCM authenticated encryption.

**Key derivation:**

```
AETHER_SECRET_KEY (environment variable)
        │
        ▼
   SHA-256 hash
        │
        ▼
  32-byte AES key
```

**Encrypt operation:**

```
plaintext
    │
    ├─── Generate random 12-byte nonce (OsRng)
    │
    ▼
AES-256-GCM encrypt(key, nonce, plaintext)
    │
    ▼
base64( nonce ‖ ciphertext )   ← stored in secrets.json
```

**Decrypt operation:**

```
base64-encoded string
    │
    ▼
base64 decode
    │
    ├─── Split: first 12 bytes = nonce
    │           remaining bytes = ciphertext
    ▼
AES-256-GCM decrypt(key, nonce, ciphertext)
    │
    ▼
plaintext
```

> 💡 **Nonce uniqueness:** Each encrypt operation generates a fresh random
> 12-byte nonce via `OsRng`. Encrypting the same plaintext twice produces
> different ciphertexts, preventing ciphertext comparison attacks.

### 6.2 Fallback key derivation

When `AETHER_SECRET_KEY` is not set:

```rust
// Deterministic key from machine identity (NOT process ID)
let hostname = env::var("HOSTNAME")
    .or_else(|_| env::var("USER"))
    .unwrap_or_else(|_| "aether-local".to_string());
let seed = format!("aether-machine-key-{}", hostname);
let key = SHA256(seed);
```

A warning is logged on every operation. The process ID is **intentionally
excluded** -- it changes on every restart, which would make secrets
unrecoverable. The hostname is stable across restarts.

> **Never use the fallback key in production.** It is deterministic and
> derivable by anyone with access to the machine hostname.

### 6.3 API authentication

The API server uses Bearer token authentication via `AETHER_API_KEY`:

```
Request:
  GET /api/workloads
  Authorization: Bearer <AETHER_API_KEY value>

Middleware checks:
  1. Is AETHER_API_KEY set? If not → allow all requests (dev mode)
  2. Is this /health or /? → always public
  3. Does Authorization header match? → allow or 401 Unauthorized
```

### 6.4 CORS policy

CORS is restricted to the server's own origin:

```rust
let origin = format!("http://{}:{}", config.host, config.port);
CorsLayer::new()
    .allow_origin(origin.parse::<HeaderValue>())
    .allow_methods([GET, POST, DELETE])
    .allow_headers(Any)
```

This prevents cross-origin requests from arbitrary websites while
allowing the built-in web dashboard to function.

### 6.5 Audit integrity

Each audit event includes an integrity hash:

```
integrity_hash = HMAC-SHA256(
    key = audit_key,
    data = "id|timestamp|action|workload|result|message"
)
```

This detects tampering -- if someone edits `audit.json` manually, the
hash will not match when verified.

### 6.6 File permissions

Sensitive files are created with restricted permissions:

| File / Directory     | Permissions | Purpose                           |
|----------------------|-------------|-----------------------------------|
| Backups              | `0o600`     | Only owner can read/write         |
| Snapshots            | `0o600`     | Only owner can read/write         |
| `secrets.json`       | Default     | Encrypted at rest (AES-256-GCM)   |

### 6.7 Input validation

Beyond the workload spec validation, Aether enforces:

- **DNS-1123 names** for all workload identifiers
- **Path traversal prevention** in file operations
- **System directory blocking** to prevent writing to protected paths
- **Request body size limit** of 2 MB on the API server

---

## 🌐 Part 7: The API Server

*Estimated time: 5 minutes*

Aether includes a full REST API server built on Axum (`src/api/mod.rs`).

### 7.1 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Axum Router                              │
│                                                                 │
│  Middleware stack (applied via tower ServiceBuilder):            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ 1. DefaultBodyLimit (2 MB)                                │  │
│  │ 2. CorsLayer (restricted to server origin)                │  │
│  │ 3. auth_middleware (Bearer token from AETHER_API_KEY)     │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Shared state: AppState {                                       │
│      state: Arc<RwLock<StateStore>>                              │
│  }                                                              │
│                                                                 │
│  40+ endpoints organized by resource                            │
└─────────────────────────────────────────────────────────────────┘
```

### 7.2 Endpoint overview

| Resource           | Method   | Endpoint                               | Description             |
|--------------------|----------|----------------------------------------|-------------------------|
| **Health**         | `GET`    | `/health`                              | Server health check     |
| **Dashboard**      | `GET`    | `/`                                    | Web dashboard           |
| **Workloads**      | `GET`    | `/api/workloads`                       | List all workloads      |
|                    | `POST`   | `/api/workloads`                       | Create workload         |
|                    | `GET`    | `/api/workloads/:name`                 | Get workload details    |
|                    | `DELETE` | `/api/workloads/:name`                 | Delete workload         |
|                    | `GET`    | `/api/workloads/:name/logs`            | Get logs                |
|                    | `POST`   | `/api/workloads/:name/start`           | Start workload          |
|                    | `POST`   | `/api/workloads/:name/stop`            | Stop workload           |
|                    | `POST`   | `/api/workloads/:name/migrate`         | Migrate workload        |
|                    | `POST`   | `/api/workloads/:name/build`           | Build image             |
| **Validation**     | `POST`   | `/api/validate`                        | Validate spec           |
| **Secrets**        | `GET`    | `/api/secrets`                         | List secrets            |
|                    | `GET`    | `/api/secrets/:name`                   | Get secret              |
|                    | `DELETE` | `/api/secrets/:name`                   | Delete secret           |
| **AI**             | `POST`   | `/api/ai/recommend`                    | Runtime recommendation  |
|                    | `GET`    | `/api/ai/profile/:name`               | Workload profiling      |
|                    | `GET`    | `/api/ai/analyze/:name`               | Log analysis            |
|                    | `GET`    | `/api/ai/migration-advice/:name/:target` | Migration advice     |
|                    | `GET`    | `/api/ai/scaling-advice`              | Scaling predictions     |
| **Operations**     | `GET`    | `/api/metrics`                         | Prometheus metrics      |
|                    | `POST`   | `/api/cost`                            | Cost estimation         |
|                    | `GET`    | `/api/backups`                         | List backups            |
|                    | `POST`   | `/api/backups`                         | Create backup           |
|                    | `GET`    | `/api/drift/:name`                    | Check drift             |
|                    | `POST`   | `/api/policy/check`                   | Policy check            |
|                    | `GET`    | `/api/audit`                          | Audit trail             |
|                    | `GET`    | `/api/events`                         | Event stream            |
|                    | `GET`    | `/api/events/summary`                 | Event summary           |

### 7.3 Response format

All API responses use a consistent JSON envelope:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

On failure:

```json
{
  "success": false,
  "data": null,
  "error": "Workload 'my-app' not found"
}
```

### 7.4 Shared state

The `StateStore` is wrapped in `Arc<RwLock<StateStore>>` to allow
concurrent reads (multiple `GET` requests) while serializing writes
(creating, deleting, or migrating workloads).

---

## 📊 Part 8: Observability & Events

*Estimated time: 5 minutes*

### 8.1 Prometheus metrics

Aether exports Prometheus-compatible metrics via the `/api/metrics` endpoint.

| Metric                               | Type        | Labels              | Description                       |
|--------------------------------------|-------------|---------------------|-----------------------------------|
| `aether_workload_builds_total`       | Counter     | runtime, status     | Total build operations            |
| `aether_workload_deployments_total`  | Counter     | runtime, status     | Total deployments                 |
| `aether_workload_running`            | Gauge       | runtime             | Currently running workloads       |
| `aether_workload_state`              | Gauge       | runtime, state      | Workload state distribution       |
| `aether_migrations_total`            | Counter     | *(per strategy)*    | Migration operations              |

These are registered in a global `prometheus::Registry` using `LazyLock`
for thread-safe initialization.

### 8.2 Event system

Events (`src/events.rs`) represent significant system occurrences with
four severity levels:

| Severity     | When used                                              |
|--------------|--------------------------------------------------------|
| `Info`       | Successful deployments, migrations, backups            |
| `Warning`    | Drift detected, secrets approaching expiry             |
| `Error`      | Failed deployments, health check failures              |
| `Critical`   | SLA violations, security incidents                     |

Events are categorized for filtering:

| Category          | Examples                                    |
|-------------------|---------------------------------------------|
| `Deployment`      | Workload started, stopped, deleted          |
| `Migration`       | Runtime migration initiated, completed      |
| `SlaViolation`    | Uptime dropped below threshold              |
| `DriftDetected`   | Live state diverged from spec               |
| `PolicyViolation` | Policy check failed                         |
| `HealthCheck`     | Health probe passed or failed               |
| `SecretRotation`  | Secret key rotated or expired               |

### 8.3 Webhook notifications

Webhooks are configured per channel with a minimum severity filter:

```bash
aether webhook add slack-alerts \
  "https://hooks.slack.com/services/T00/B00/xxx" \
  --severity warning
```

Failed deliveries enter a retry queue with exponential backoff. View
and manage the queue:

```bash
aether webhook queue       # View pending retries
aether webhook flush       # Force-retry all queued
```

### 8.4 Audit trail

The audit system (`src/audit.rs`) records every state-changing operation:

| Action          | Recorded when                             |
|-----------------|-------------------------------------------|
| `Build`         | Image built                               |
| `Deploy`        | Workload deployed                         |
| `Start`/`Stop`  | Workload started or stopped               |
| `Delete`        | Workload deleted                          |
| `Migrate`       | Runtime migration executed                |
| `Scale`         | Replica count changed                     |
| `ConfigChange`  | Configuration updated                     |
| `BackupCreate`  | Backup created                            |
| `BackupRestore` | Backup restored                           |
| `PolicyCheck`   | Policy evaluation performed               |
| `DriftDetected` | Configuration drift found                 |

Each event includes an `integrity_hash` (SHA-256) computed over the event
fields. This enables tamper detection when reviewing the audit trail:

```bash
aether audit --last 50 --workload api-service
```

### 8.5 Health history

The orchestrator watch loop (`aether orchestrate watch`) records health
observations over time, enabling:

- **Uptime calculations** for SLA compliance
- **Restart counting** for circuit breaker decisions
- **Trend analysis** for scaling recommendations

---

## 🏁 Summary

### The full request lifecycle

When you run `aether run`, here is the complete sequence of events:

```
1. CLI parse         clap parses args → loads workload.yaml
                     │
2. Validate          Workload::from_file() → validate()
                     │
3. Policy check      PolicyEngine evaluates rules (if enforce_on_deploy)
                     │
4. Engine decide     Engine::decide() → rule chain → RuntimeKind
                     │
5. Create adapter    create_runtime_ns(kind, namespace)
                     │
6. Build             Runtime::build(spec) → Image
                     │
7. Deploy            Runtime::run(image, spec) → Instance
                     │
8. State update      StateStore::upsert() → atomic write to state.json
                     │
9. Audit log         AuditTrail::record(Deploy, workload, Success)
                     │
10. Metrics record   WORKLOAD_DEPLOYMENTS_TOTAL.inc()
                     WORKLOAD_RUNNING.inc()
                     │
11. Event emit       EventBus::emit(Deployment, Info, "Deployed to podman")
                     │
12. Webhook notify   WebhookManager::notify() → POST to configured channels
```

### How to extend Aether

#### Adding a new runtime adapter

1. Create `src/adapters/my_runtime.rs`
2. Implement the `Runtime` trait (7 async methods)
3. Add a variant to `RuntimeKind` enum in `src/runtime.rs`
4. Register the adapter in `create_runtime_ns()`
5. Add a rule to the decision engine if appropriate
6. Add the runtime type to `RuntimeType` and `RuntimePreference` enums in `src/spec.rs`

#### Writing a plugin

Plugins are external binaries that implement the Runtime trait over
a JSON-RPC protocol on stdin/stdout:

1. Create a manifest JSON (`name`, `version`, `runtime_kind`, `command`, `capabilities`)
2. Place it in `~/.aether/plugins/`
3. Implement handlers for `BuildRequest`, `RunRequest`, `StopRequest`, etc.
4. Each IPC call has a 60-second timeout

See [Tutorial 3: Plugin System](./03-advanced-features.md#-plugin-system) for the full protocol reference.

### Further reading

| Document                                                                 | Description                                     |
|--------------------------------------------------------------------------|------------------------------------------------|
| [Tutorial 1 - Beginner Deployment](./01-beginner-deployment.md)          | First deployment walkthrough                    |
| [Tutorial 2 - Intermediate Workflows](./02-intermediate-workflows.md)    | Compose files, migrations, output formats       |
| [Tutorial 3 - Advanced Features](./03-advanced-features.md)              | Policies, secrets, drift, plugins               |
| [CLI Reference](../guides/cli/CLI-Reference.md)                         | Complete command reference                      |
| [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md)       | Pre/post migration procedures                   |

---

> 📚 **Full documentation:** [CLI Reference](../guides/cli/CLI-Reference.md)
> 🏷 **License:** Proprietary HyperSDK
