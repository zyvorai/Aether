---
hero:
  eyebrow: TUTORIALS
  title: '🔄 Tutorial 2: Intermediate Workflows'
  tone: amber
  swatches:
    - {label: "45–60 min", tone: amber}
    - {label: "Level: Intermediate", tone: sky}
    - {label: "Prereq: Tutorial 1", tone: violet}
---

## 📑 Table of Contents

- [Multi-Workload Compose Files](#-multi-workload-compose-files)
- [Compose Up and Down](#-compose-up-and-down)
- [Runtime Comparison](#-runtime-comparison)
- [Migration Between Runtimes](#-migration-between-runtimes)
- [Output Formats](#-output-formats)
- [Dry-Run Mode](#-dry-run-mode)
- [Health Monitoring](#-health-monitoring)
- [Watch Mode for Auto-Redeploy](#-watch-mode-for-auto-redeploy)
- [Next Steps](#-next-steps)

---

## 🗂 Multi-Workload Compose Files

Real applications are rarely a single container. Aether compose files let you
define, deploy, and manage multiple workloads as a single unit with dependency
ordering, runtime overrides, and per-workload environment variables.

### Compose file schema

Create `aether-compose.yaml` in your project root:

```yaml
# aether-compose.yaml — Multi-workload deployment
version: "1"

workloads:
  # ── Database layer ────────────────────────────────────────────
  database:
    spec: ./specs/database.yaml
    runtime: container
    env:
      POSTGRES_DB: myapp
      POSTGRES_USER: admin
      POSTGRES_PASSWORD: "${DB_PASSWORD}"

  # ── Cache layer ───────────────────────────────────────────────
  cache:
    spec: ./specs/cache.yaml
    runtime: container
    env:
      MAXMEMORY: "256mb"

  # ── API backend ──────────────────────────────────────────────
  api:
    spec: ./specs/api.yaml
    runtime: kube
    depends_on:
      - database
      - cache
    env:
      DATABASE_URL: "postgres://admin:${DB_PASSWORD}@database:5432/myapp"
      REDIS_URL: "redis://cache:6379"
      LOG_LEVEL: info

  # ── Worker process ───────────────────────────────────────────
  worker:
    spec: ./specs/worker.yaml
    depends_on:
      - database
      - cache
    env:
      QUEUE_CONCURRENCY: "10"

  # ── Web frontend ─────────────────────────────────────────────
  web:
    spec: ./specs/web.yaml
    depends_on:
      - api
    env:
      API_URL: "http://api:8080"
      NODE_ENV: production
```

### Key compose features

| Feature          | Description                                                     |
|------------------|-----------------------------------------------------------------|
| `spec`           | Path to the workload YAML spec file (relative to compose file)  |
| `runtime`        | Optional runtime override per workload                          |
| `depends_on`     | Deployment ordering -- listed workloads start first             |
| `env`            | Extra environment variables injected at deploy time             |

### Dependency resolution

Aether uses Kahn's algorithm for topological sorting. Given the compose file
above, the deployment order is:

```
1. database, cache    (no dependencies -- deployed in parallel)
2. api, worker        (depend on database + cache)
3. web                (depends on api)
```

> ⚠️ **Circular dependencies are detected and rejected:**
>
> ```
> Error: circular dependency detected in compose workloads
> ```

---

## 🚢 Compose Up and Down

### Validate the compose file

```bash
aether compose validate
```

Or with a custom path:

```bash
aether compose validate ./deploy/aether-compose.yaml
```

### Deploy all workloads

```bash
aether compose up
```

Options:

```bash
# Override runtime for ALL workloads
aether compose up --runtime podman

# Preview the deployment plan
aether compose up --dry-run

# Use a custom compose file
aether compose up ./deploy/aether-compose.yaml
```

### Tear down all workloads

```bash
aether compose down
```

This stops and removes all workloads defined in the compose file, in reverse
dependency order.

---

## ⚖️ Runtime Comparison

Before committing to a runtime, compare your workload across all three targets:

```bash
aether compare
```

Sample output:

```
┌───────────────┬──────────┬───────┬──────────────┬───────────────────────────┐
│ Runtime       │ Score    │ Cost  │ Capabilities │ Limitations               │
├───────────────┼──────────┼───────┼──────────────┼───────────────────────────┤
│ 🐳 Podman     │  85/100  │ $0    │ Fast startup │ Single host only          │
│ ☸️ Kubernetes  │  92/100  │ $45   │ HA, scaling  │ Cluster required          │
│ 🖥️ KubeVirt   │  78/100  │ $62   │ GPU, VMs     │ Higher overhead           │
└───────────────┴──────────┴───────┴──────────────┴───────────────────────────┘
```

### AI-powered recommendations

```bash
aether recommend
```

The recommendation engine scores each runtime based on:

- Workload resource requirements
- Network and persistence needs
- GPU requirements
- Cost optimization
- Availability and scaling needs

---

## 🔀 Migration Between Runtimes

Aether supports zero-downtime migration between any two runtimes. Three
strategies are available:

### Strategy comparison

| Strategy      | Downtime | Risk     | Speed   | Use Case                          |
|---------------|----------|----------|---------|-----------------------------------|
| `immediate`   | Yes      | Medium   | Fast    | Dev/test, non-critical workloads  |
| `blue-green`  | No       | Low      | Medium  | Production, stateless services    |
| `rolling`     | No       | Lowest   | Slow    | Critical services, gradual rollout|

---

### Strategy 1: Immediate

Stops the source, then starts on the target. Fastest but incurs downtime.

```bash
aether migrate hello-web kube --strategy immediate
```

**Flow:**

```
1. Stop source instance on Podman
2. Wait for graceful shutdown (5s)
3. Build image for Kubernetes
4. Deploy to Kubernetes
5. Validate health (10s delay)
6. Delete source instance
7. Update state
```

---

### Strategy 2: Blue-Green (default)

Deploys to the target (green) while the source (blue) is still running.
Switches traffic only after the green deployment passes health checks.

```bash
aether migrate hello-web kube --strategy blue-green
```

**Flow:**

```
1. Deploy to target (green) while source (blue) runs
2. Wait for green deployment health check
3. Switch traffic to green
4. Wait for connection draining
5. Stop and delete blue (source)
6. Update state
```

---

### Strategy 3: Rolling

Gradual traffic shift with exponential-backoff health validation at each step.

```bash
aether migrate hello-web kube --strategy rolling
```

**Flow:**

```
Phase 1: Deploy to target runtime
Phase 2: Validate with exponential-backoff retries (3 attempts, 2s base)
Phase 3: Gradual traffic shift (25% → 50% → 75% → 100%)
         ↳ Health check at each step — rollback on failure
Phase 4: Cleanup source deployment
```

---

### Migration flags

| Flag                 | Default       | Description                              |
|----------------------|---------------|------------------------------------------|
| `--strategy <name>`  | `blue-green`  | Migration strategy                       |
| `--no-validation`    | `false`       | Skip the post-migration validation delay |
| `--no-rollback`      | `false`       | Disable automatic rollback on failure    |
| `--dry-run`          | `false`       | Show what would happen, do not execute   |

### Rollback

If a migration fails mid-flight (and `--no-rollback` was not set), Aether
automatically rolls back to the source runtime.

To manually rollback a workload to its latest snapshot:

```bash
aether rollback hello-web
```

---

## 🖨 Output Formats

Every Aether command supports four output modes:

### Table (default)

```bash
aether list
```

```
┌─────────────┬─────────┬─────────┐
│ Name        │ Runtime │ State   │
├─────────────┼─────────┼─────────┤
│ hello-web   │ podman  │ running │
│ api-svc     │ kube    │ running │
└─────────────┴─────────┴─────────┘
```

### JSON

```bash
aether --output json list
# or
aether --json list
```

```json
{
  "workloads": [
    {
      "name": "hello-web",
      "runtime": "podman",
      "state": "running"
    }
  ]
}
```

### YAML

```bash
aether --output yaml status hello-web
```

```yaml
workload: hello-web
runtime: podman
state: running
ready: true
restarts: 0
```

### Wide

```bash
aether --output wide list
```

Adds extra columns such as image, created-at, spec path, and migration history.

### Quiet mode (errors only)

```bash
aether --quiet run
```

Suppresses all output except errors. Useful for CI pipelines where you only
care about exit codes.

---

## 🧪 Dry-Run Mode

Any mutating command can be previewed without side effects:

```bash
# Preview a deployment
aether --dry-run run

# Preview a migration
aether --dry-run migrate hello-web kube --strategy rolling

# Preview a stop
aether --dry-run stop hello-web

# Preview a delete
aether --dry-run delete hello-web

# Preview a compose deployment
aether compose up --dry-run
```

Dry-run output is prefixed with `[dry-run]`:

```
[dry-run] Would migrate 'hello-web' to kube using rolling strategy
```

---

## 💓 Health Monitoring

### View health history

```bash
aether health hello-web
```

Options:

```bash
# Show last 50 health records
aether health hello-web --last 50

# Show summary only (uptime %, restart count, last state)
aether health hello-web --summary
```

Sample summary:

```
  Workload          hello-web
  Total Checks      142
  Ready Checks      140
  Uptime            98.59%
  Last Restarts     1
  Last State        running
```

### Continuous health monitoring

Register a workload with the orchestrator and run health checks:

```bash
# Register for monitoring
aether orchestrate register hello-web --runtime podman

# Run a single health check
aether orchestrate health-check

# Show health status of all workloads
aether orchestrate status

# Show summary
aether orchestrate summary
```

### Continuous watch mode

```bash
# Check every 30 seconds (default)
aether orchestrate watch

# Custom interval
aether orchestrate watch --interval 60
```

---

## 👁 Watch Mode for Auto-Redeploy

Watch your spec file and automatically redeploy when it changes:

```bash
aether watch
```

Options:

```bash
# Watch with a specific runtime
aether watch --runtime podman

# Watch a different spec file
aether --spec ./custom.yaml watch
```

Aether monitors the workload spec file for changes. When the file is saved,
it:

1. Re-validates the spec
2. Stops the running instance
3. Rebuilds the image
4. Redeploys with the updated configuration
5. Verifies health

This creates a hot-reload development loop similar to `cargo watch` or
`nodemon`.

---

## 🎯 Next Steps

You have now mastered compose files, migration strategies, output formats, and
health monitoring. Continue with the advanced features:

| Tutorial                                                                 | Topics                                          |
|--------------------------------------------------------------------------|------------------------------------------------|
| [03 - Advanced Features](./03-advanced-features.md)                      | Policies, secrets, drift detection, plugins     |
| [CLI Reference](../guides/cli/CLI-Reference.md)                         | Complete command reference                      |
| [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md)       | Pre/post migration procedures                   |

### Quick wins to try now

```bash
# Estimate costs across cloud providers
aether cost --provider all

# View the diff between spec, state, and live runtime
aether diff hello-web

# Deploy all specs in a directory
aether deploy ./specs/

# Batch deploy with fail-fast
aether deploy ./specs/ --fail-fast

# Check SLA compliance
aether sla check hello-web --uptime 99.9 --latency 50 --error-rate 0.01
```

---

> 📚 **Full documentation:** [CLI Reference](../guides/cli/CLI-Reference.md)
> 🏷 **License:** Proprietary HyperSDK
