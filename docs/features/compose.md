# 🎼 Compose: Multi-Workload Deployments

> Deploy entire application stacks with a single command using dependency-ordered orchestration.

---

## 📑 Table of Contents

- [What is aether-compose.yaml?](#-what-is-aether-composeyaml)
- [Compose Spec Format](#-compose-spec-format)
- [Dependency Resolution](#-dependency-resolution)
- [Commands](#-commands)
- [Examples](#-examples)
- [Dry-Run Mode](#-dry-run-mode)
- [Policy Gate Integration](#-policy-gate-integration)
- [REST API](#-rest-api)
- [Cross-References](#-cross-references)

---

## 📄 What is aether-compose.yaml?

An `aether-compose.yaml` file groups multiple workloads into a single deployment unit. Instead of deploying each service individually, you declare them all in one file with:

- **Dependency ordering** -- workloads start in the correct sequence
- **Runtime overrides** -- pin specific workloads to specific runtimes
- **Environment injection** -- pass per-workload env vars at deploy time
- **Validation** -- catch circular dependencies and missing references before anything runs

The default file name is `aether-compose.yaml`, but you can pass any path to the compose subcommands.

---

## 📋 Compose Spec Format

### Full Schema

```yaml
version: "1"
workloads:
  <workload-name>:
    spec: <path-to-workload-yaml>
    runtime: <optional-runtime-override>
    depends_on:
      - <other-workload-name>
    env:
      KEY: value
```

### Field Reference

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `version` | string | Yes | -- | Schema version. Currently `"1"`. |
| `workloads` | map | Yes | -- | Named workload entries (keys are logical names). |
| `workloads.<name>.spec` | path | Yes | -- | Path to the workload YAML spec file (relative to compose file). |
| `workloads.<name>.runtime` | string | No | auto | Runtime override: `container`, `kube`, `kubevirt`, `metal`. When omitted, the decision engine selects automatically. |
| `workloads.<name>.depends_on` | list | No | `[]` | Names of workloads that must be started before this one. |
| `workloads.<name>.env` | map | No | `{}` | Extra environment variables injected at deploy time. |

### Runtime Override Values

| Value | Runtime | Icon |
|---|---|---|
| `container` / `podman` | Podman | 🐳 |
| `kube` / `kubernetes` / `k8s` | Kubernetes | ☸️ |
| `kubevirt` / `vm` | KubeVirt | 🖥️ |
| `metal` / `metal3` / `bare-metal` | Metal3 | 🖧 |

### Minimal Example

```yaml
version: "1"
workloads:
  web:
    spec: ./web.yaml
```

### Full-Featured Example

```yaml
version: "1"
workloads:
  database:
    spec: ./db.yaml
    runtime: container
    env:
      POSTGRES_DB: mydb
      POSTGRES_USER: admin
  api:
    spec: ./api.yaml
    runtime: kube
    depends_on:
      - database
    env:
      LOG_LEVEL: debug
  web:
    spec: ./web.yaml
    depends_on:
      - api
```

---

## 🔀 Dependency Resolution

aether uses **Kahn's algorithm** (topological sort) to determine the correct startup order.

### Algorithm Steps

1. Build an **in-degree map** for all workloads (count of incoming dependencies)
2. Seed a queue with **zero-dependency workloads** (sorted alphabetically for determinism)
3. Process each node: add to result, decrement dependents' in-degrees
4. If a dependent's in-degree reaches zero, enqueue it (also sorted for determinism)
5. If the result is shorter than the workload count, **a cycle exists**

### Resolution Rules

| Rule | Behavior |
|---|---|
| No dependencies | Start first (alphabetical tiebreaker among peers) |
| Linear chain | Strict sequential order |
| Diamond pattern | Shared dependency starts first, dependents follow when ready |
| Circular reference | Error: `"circular dependency detected in compose workloads"` |
| Missing reference | Error: `"workload 'X' depends on 'Y' which is not defined"` |

### Example: Diamond Dependency

```
database (0 deps) ──► api (depends on database)
                  └──► worker (depends on database)
                           │                  │
                           └──► web (depends on api, worker)
```

**Resolved order:** `database` → `api` → `worker` → `web`

### Example: Linear Chain

```
database → api → web
```

**Resolved order:** `database` → `api` → `web`

---

## ⚡ Commands

### `compose validate`

Validate a compose file for structural correctness without deploying anything.

```bash
aether compose validate                           # default: aether-compose.yaml
aether compose validate ./my-stack.yaml           # explicit path
```

**Checks performed:**

| Check | Error on failure |
|---|---|
| YAML syntax | `"failed to parse compose file"` |
| `depends_on` references | `"workload 'X' depends on 'Y' which is not defined"` |
| Circular dependencies | `"circular dependency detected in compose workloads"` |

**Output includes:**
- Workload count
- Computed deploy order (topological)
- Any validation errors

---

### `compose up`

Deploy all workloads from a compose file in dependency order.

```bash
aether compose up                                    # default file
aether compose up ./production-stack.yaml            # explicit path
aether compose up --runtime kube                     # override runtime for all
aether compose up --dry-run                          # preview without executing
```

| Flag | Short | Description |
|---|---|---|
| `--runtime` | `-r` | Override runtime for **all** workloads (ignores per-workload `runtime` fields) |
| `--dry-run` | -- | Show the deployment plan without executing |

---

### `compose down`

Stop all workloads defined in a compose file in **reverse dependency order** (dependents stop first).

```bash
aether compose down                                  # default file
aether compose down ./production-stack.yaml          # explicit path
```

---

## 📦 Examples

### 3-Tier Application (db → api → web)

**Directory layout:**

```
my-app/
├── aether-compose.yaml
├── db.yaml
├── api.yaml
└── web.yaml
```

**aether-compose.yaml:**

```yaml
version: "1"
workloads:
  database:
    spec: ./db.yaml
    runtime: container
    env:
      POSTGRES_DB: myapp
      POSTGRES_USER: admin
      POSTGRES_PASSWORD: "${DB_PASSWORD}"

  api:
    spec: ./api.yaml
    runtime: kube
    depends_on:
      - database
    env:
      DATABASE_URL: "postgres://admin:${DB_PASSWORD}@database:5432/myapp"
      LOG_LEVEL: info
      JWT_SECRET: "${JWT_SECRET}"

  web:
    spec: ./web.yaml
    depends_on:
      - api
    env:
      API_BASE_URL: "http://api:8080"
      NODE_ENV: production
```

**Deployment workflow:**

```bash
# 1. Validate first
aether compose validate
# ✅ Valid compose file with 3 workloads
# Deploy order: database → api → web

# 2. Preview the plan
aether compose up --dry-run

# 3. Deploy the stack
aether compose up

# 4. Tear down (web stops first, then api, then database)
aether compose down
```

---

### Microservices with Shared Dependencies

```yaml
version: "1"
workloads:
  redis:
    spec: ./infra/redis.yaml
    runtime: container

  postgres:
    spec: ./infra/postgres.yaml
    runtime: container
    env:
      POSTGRES_DB: services

  auth-service:
    spec: ./services/auth.yaml
    runtime: kube
    depends_on:
      - redis
      - postgres

  user-service:
    spec: ./services/users.yaml
    runtime: kube
    depends_on:
      - postgres

  gateway:
    spec: ./services/gateway.yaml
    depends_on:
      - auth-service
      - user-service
    env:
      RATE_LIMIT: "1000"
```

**Resolved deploy order:** `postgres` → `redis` → `auth-service` → `user-service` → `gateway`

---

### Mixed Runtimes with Environment Injection

```yaml
version: "1"
workloads:
  ml-model:
    spec: ./ml/model-server.yaml
    runtime: metal
    env:
      MODEL_PATH: /models/v3
      GPU_MEMORY: "16Gi"

  api-gateway:
    spec: ./gateway.yaml
    runtime: kube
    depends_on:
      - ml-model
    env:
      ML_ENDPOINT: "http://ml-model:9090/predict"
      CACHE_TTL: "300"
```

---

## 🔍 Dry-Run Mode

Preview the deployment plan without executing anything:

```bash
aether compose up --dry-run
```

Dry-run output shows:

- ✅ The resolved deploy order
- ✅ Runtime selection for each workload (auto-decided or overridden)
- ✅ Environment variables that will be injected
- ✅ Any policy violations (see below)

Combine with the global `--dry-run` flag for consistent behavior:

```bash
aether --dry-run compose up
```

---

## 🛡️ Policy Gate Integration

When the policy gate is enabled, each workload in the compose file is evaluated against your active policy set **before** deployment begins. If any workload fails a policy check, the entire `compose up` is aborted.

```bash
# Deploy with policy enforcement (default behavior)
aether compose up

# Skip policy checks (use with caution)
aether --skip-policy compose up
```

Policy checks evaluate each workload's spec individually against the configured rule set. See the [Security Guide](./security.md) for the full list of 10 policy rule types.

---

## 🌐 REST API

### POST `/api/compose/validate`

Validate a compose spec via the REST API.

**Request:** Raw YAML body (Content-Type: `text/plain` or `application/x-yaml`)

```bash
curl -X POST http://localhost:8080/api/compose/validate \
  -H "Content-Type: text/plain" \
  -d @aether-compose.yaml
```

**Response (valid):**

```json
{
  "success": true,
  "data": {
    "valid": true,
    "workload_count": 3,
    "deploy_order": ["database", "api", "web"]
  },
  "error": null
}
```

**Response (invalid -- missing dependency):**

```json
{
  "success": false,
  "data": null,
  "error": "workload 'api' depends on 'database' which is not defined in the compose file"
}
```

**Response (invalid -- circular dependency):**

```json
{
  "success": false,
  "data": null,
  "error": "circular dependency detected in compose workloads"
}
```

**Response (invalid YAML):**

```json
{
  "success": false,
  "data": null,
  "error": "Invalid YAML: ..."
}
```

---

## 🔗 Cross-References

| Document | Relevance |
|---|---|
| [Security & Policy Guide](./security.md) | Policy gate rules and `--skip-policy` |
| [Plugin System](./plugins.md) | Custom runtimes available as compose targets |
| [Health Monitoring](./health-monitoring.md) | Health tracking for deployed workloads |
| [API Reference](../reference/api/API-Reference.md) | Full REST API documentation |
| [Quick Reference](../quick-reference/QUICK_REFERENCE.md) | Command cheat sheet |
