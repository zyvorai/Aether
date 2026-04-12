# ⚙️ Tutorial 3: Advanced Features

> **Estimated time:** 60--90 minutes
> **Level:** Advanced
> **Prerequisites:** [Tutorial 2 - Intermediate Workflows](./02-intermediate-workflows.md)
> **License:** Proprietary HyperSDK

---

## 📑 Table of Contents

- [Policy Engine](#-policy-engine)
- [Secrets Management (AES-256-GCM)](#-secrets-management-aes-256-gcm)
- [Drift Detection and Auto-Reconciliation](#-drift-detection-and-auto-reconciliation)
- [Plugin System](#-plugin-system)
- [Webhook Notifications with Retry Queue](#-webhook-notifications-with-retry-queue)
- [Background Reconciliation Loop](#-background-reconciliation-loop)
- [Cost Estimation Across Providers](#-cost-estimation-across-providers)
- [AI-Powered Recommendations](#-ai-powered-recommendations)
- [Next Steps](#-next-steps)

---

## 🛡 Policy Engine

The policy engine enforces governance rules *before* deployment, catching
violations early. Policies cover resource limits, naming conventions, security
requirements, and compliance checks.

### Built-in policy sets

| Policy Set      | Description                                      | Use Case          |
|-----------------|--------------------------------------------------|-------------------|
| `production`    | Strict limits: health probes, TLS, resource caps | Prod environments |
| `development`   | Relaxed: larger resource limits, fewer checks    | Dev/test          |
| Custom file     | Your own rules in YAML                           | Enterprise        |

### Running a policy check

```bash
# Check against production policies (default)
aether policy-check

# Check against development policies
aether policy-check --policy development

# Check against a custom policy file
aether policy-check --policy ./my-policies.yaml
```

Sample output:

```
Policy Check: production (12 rules evaluated)

  ✔ PASS  resource-limits     CPU within maximum (2 <= 16)
  ✔ PASS  resource-limits     Memory within maximum (4Gi <= 64Gi)
  ✔ PASS  naming-convention   Name matches prefix pattern
  ✗ FAIL  security            Health probes required but not configured
  ⚠ WARN  best-practice       Consider enabling persistence for stateful data

Result: 1 violation, 1 warning
```

### Production policy rules

The built-in `production` policy enforces:

| Rule                      | Check Type             | Threshold / Requirement        |
|---------------------------|------------------------|--------------------------------|
| Max CPU                   | `MaxCpu`               | 16 cores                       |
| Max Memory                | `MaxMemoryGi`          | 64 GiB                         |
| Max Storage               | `MaxStorageGi`         | 500 GiB                        |
| Max GPU                   | `MaxGpu`               | 8 GPUs                         |
| Health Probes Required    | `RequireHealthProbes`  | Liveness + readiness required  |
| TLS Required              | `RequireTls`           | Ingress must use TLS           |
| Owner Required            | `RequireOwner`         | `metadata.owner` must be set   |
| Resource Limits Required  | `RequireResourceLimits`| CPU + memory must be specified |
| Minimum Replicas          | `MinReplicas`          | At least 2 replicas            |
| Name Prefix               | `NamePrefix`           | Must match organizational prefix|
| Disallow Runtime          | `DisallowRuntime`      | Block specific runtimes        |

### Custom policies

Create a YAML file with your own rules:

```yaml
name: my-org-policies
description: "Custom policies for ACME Corp"
enabled: true
rules:
  - name: max-cpu
    check:
      MaxCpu: 8
    severity: Error
    message: "CPU must not exceed 8 cores in this cluster"

  - name: require-health
    check: RequireHealthProbes
    severity: Error
    message: "All workloads must define liveness and readiness probes"

  - name: no-metal
    check:
      DisallowRuntime: metal
    severity: Error
    message: "Bare-metal runtime is not available in this environment"

  - name: naming
    check:
      NamePrefix: "acme-"
    severity: Warning
    message: "Workload names should start with 'acme-'"
```

### Policy gate on deploy

Policies can be enforced automatically on every `run` and `deploy` command.
This is configured in `~/.aether/config.yaml`:

```yaml
policy:
  enforce_on_deploy: true
  default_policy: production
```

When a violation is detected, the deployment is blocked:

```
Error: Policy violation: Health probes required but not configured
Hint: Add health.liveness and health.readiness to your workload spec,
      or use --skip-policy to bypass (not recommended for production).
```

To bypass in development:

```bash
aether --skip-policy run
```

---

## 🔐 Secrets Management (AES-256-GCM)

Aether includes a built-in secrets manager with AES-256-GCM encryption,
access auditing, rotation policies, and expiry alerts.

### Set the encryption key

```bash
# Production: set a strong key via environment variable
export AETHER_SECRET_KEY="your-32-char-production-key-here!"
```

> ⚠️ **Without `AETHER_SECRET_KEY`**, secrets are stored with XOR obfuscation
> (dev-only). A warning is displayed on every operation. **Never use the default
> key in production.**

| Encryption Mode   | When Active                        | Security Level |
|-------------------|------------------------------------|----------------|
| AES-256-GCM       | `AETHER_SECRET_KEY` is set      | Production     |
| XOR Obfuscation   | No key set (dev-only)              | Insecure       |
| Vault Reference   | External vault integration         | Enterprise     |

### Create a secret

```bash
aether secrets create db-credentials --namespace production
```

### Set key-value pairs

```bash
aether secrets set db-credentials username admin
aether secrets set db-credentials password "s3cur3-p@ssw0rd!"
aether secrets set db-credentials connection-string "postgres://admin:s3cur3@db:5432/app"
```

### Retrieve a secret value

```bash
aether secrets get db-credentials password
```

The value is decrypted on-the-fly and the access is logged in the audit trail.

### List all secrets

```bash
aether secrets list
```

```
┌─────────────────┬────────────┬──────┬─────────────────────┬──────────┐
│ Name            │ Namespace  │ Keys │ Updated             │ Rotation │
├─────────────────┼────────────┼──────┼─────────────────────┼──────────┤
│ db-credentials  │ production │ 3    │ 2026-04-11T10:30:00 │ OK       │
│ api-keys        │ staging    │ 2    │ 2026-04-10T14:00:00 │ ⚠ Yes    │
└─────────────────┴────────────┴──────┴─────────────────────┴──────────┘
```

### Rotation audit

```bash
aether secrets audit
```

Checks all secrets against their rotation policies and reports:

- **Critical:** Key exceeded maximum age (default 365 days)
- **Warning:** Key approaching expiry (default 14 days notice)
- **OK:** Key within rotation interval (default 90 days)

### Rotation policy defaults

| Parameter              | Default | Description                           |
|------------------------|---------|---------------------------------------|
| `interval_days`        | 90      | Rotate every N days                   |
| `max_age_days`         | 365     | Force rotation after N days           |
| `notify_before_days`   | 14      | Alert N days before max age           |

### How encryption works under the hood

1. **Key derivation:** The `AETHER_SECRET_KEY` value is hashed with SHA-256
   to produce a 32-byte AES key
2. **Nonce generation:** A random 12-byte nonce is generated for each encrypt
   operation using `OsRng`
3. **Encryption:** AES-256-GCM authenticated encryption
4. **Storage:** The nonce is prepended to the ciphertext, then base64-encoded
5. **Decryption:** Nonce is extracted, then AES-256-GCM decrypts and verifies
   authenticity

> 💡 Encrypting the same plaintext twice produces different ciphertexts because
> each operation uses a unique random nonce.

---

## 🔍 Drift Detection and Auto-Reconciliation

Drift occurs when the live state of a workload diverges from the desired spec.
Common causes include manual changes, runtime updates, or resource reclamation.

### Detect drift

```bash
aether drift hello-web
```

Sample output:

```
Drift Report: hello-web

  ┌────────────┬──────────────────┬──────────────┬──────────────┬──────────┐
  │ Category   │ Field            │ Expected     │ Actual       │ Severity │
  ├────────────┼──────────────────┼──────────────┼──────────────┼──────────┤
  │ Resources  │ cpu              │ 2            │ 1            │ WARNING  │
  │ Image      │ image_tag        │ v2.1.0       │ v2.0.0       │ CRITICAL │
  │ Network    │ service_port     │ 80           │ 8080         │ WARNING  │
  └────────────┴──────────────────┴──────────────┴──────────────┴──────────┘

  Severity: CRITICAL
  Reconciliation plan: 3 actions required
```

### Drift categories

| Category        | What is compared                                 |
|-----------------|--------------------------------------------------|
| `Runtime`       | Running on the expected runtime                   |
| `Image`         | Container image tag/digest                        |
| `Resources`     | CPU, memory, storage allocations                  |
| `Network`       | Ports, service type, ingress                      |
| `Configuration` | Environment variables, config maps                |
| `Scaling`       | Replica count, autoscaling settings               |
| `Health`        | Probe definitions and thresholds                  |

### Auto-reconcile drift

```bash
aether drift hello-web --reconcile
```

This generates and executes a reconciliation plan to bring the workload back
in sync with the desired spec. Actions may include:

- Redeploying with the correct image
- Updating resource allocations
- Reconfiguring network settings
- Adjusting replica count

### Compare spec vs state vs live

For a three-way diff (spec file vs stored state vs live runtime):

```bash
aether diff hello-web
```

---

## 🧩 Plugin System

Extend Aether with custom runtimes through the plugin system. Plugins
communicate via a JSON-RPC style protocol over stdin/stdout.

### Plugin manifest format

Plugins are described by a JSON manifest:

```json
{
  "name": "wasm-runtime",
  "version": "1.0.0",
  "runtime_kind": "wasm",
  "command": "/usr/local/bin/aether-wasm-plugin",
  "capabilities": ["build", "run", "stop", "status", "delete", "list"]
}
```

| Field          | Description                                         |
|----------------|-----------------------------------------------------|
| `name`         | Plugin identifier (registry key)                    |
| `version`      | Semantic version                                    |
| `runtime_kind` | Custom runtime type this plugin provides            |
| `command`      | Path to the plugin binary                           |
| `capabilities` | Operations the plugin supports                      |

### Discover plugins

Place manifests in `~/.aether/plugins/` as `*.json` files:

```bash
aether plugin discover
```

```
Discovered 2 plugins:
  wasm-runtime v1.0.0 (capabilities: build, run, stop, status)
  firecracker  v0.3.1 (capabilities: run, stop, status, list)
```

### Register a plugin manually

```bash
aether plugin register ./my-plugin-manifest.json
```

### List registered plugins

```bash
aether plugin list
```

### Remove a plugin

```bash
aether plugin remove wasm-runtime
```

### Plugin protocol

Plugins communicate via JSON messages on stdin/stdout. Each message has a
`type` field:

| Request            | Response             | Description                |
|--------------------|----------------------|----------------------------|
| `BuildRequest`     | `BuildResponse`      | Build an image from spec   |
| `RunRequest`       | `RunResponse`        | Start a workload instance  |
| `StopRequest`      | `StopResponse`       | Stop a running instance    |
| `StatusRequest`    | `StatusResponse`     | Query instance status      |
| `DeleteRequest`    | `DeleteResponse`     | Delete an instance         |
| `ListRequest`      | `ListResponse`       | List managed instances     |

Example `BuildRequest`:

```json
{
  "type": "BuildRequest",
  "spec_json": "{\"apiVersion\":\"aether/v1\",\"kind\":\"Workload\",...}"
}
```

Example `BuildResponse`:

```json
{
  "type": "BuildResponse",
  "image_json": "{\"name\":\"my-app\",\"tag\":\"latest\",\"runtime\":\"wasm\"}"
}
```

### Plugin runtime lifecycle

When a plugin is registered, it acts as a full runtime implementation. Aether
communicates with the plugin binary via JSON-RPC over stdin/stdout:

```
aether → stdin  → {"type":"RunRequest","image_json":"...","spec_json":"..."}
plugin    → stdout → {"type":"RunResponse","instance_json":"..."}
```

Each IPC call has a **60-second timeout**. If the plugin binary doesn't respond
in time, Aether terminates the process and returns an error.

**Capability checking** runs before every operation. Calling `build` on a plugin
that only supports `["run", "stop"]` produces:

```
Error: Plugin 'wasm-runtime' does not support 'build' (capabilities: run, stop)
```

---

## 🚨 Alert Rules and Automated Alerting

The `orchestrate watch` loop evaluates alert rules against live system metrics
on every monitoring cycle. When conditions are met, events are emitted and
notifications are sent via configured webhook channels.

### Supported alert conditions

| Condition | Triggers When |
|---|---|
| `SlaUptimeBelow(99.9)` | Any workload's uptime drops below the threshold |
| `ExcessiveRestarts(5)` | Any workload exceeds the restart count |
| `DriftDetected` | Configuration drift is found |
| `PolicyViolation` | Active policy violations exist |
| `SecretExpiring(14)` | Any secret expires within N days |

### How it works

Each watch cycle:

1. Collects SLA uptimes and restart counts from health history
2. Evaluates all enabled alert rules against current metrics
3. Respects per-rule **cooldown** (prevents repeated firing)
4. Emits events for triggered rules
5. Sends webhook notifications for fired alerts

### Viewing fired alerts

```bash
# View recent alert events
aether events --severity warning

# View event summary
aether events --summary
```

---

## 📡 Webhook Notifications with Retry Queue

Configure webhook endpoints to receive notifications about workload events.

### Add a webhook channel

```bash
aether webhook add slack-alerts \
  "https://hooks.slack.com/services/T00/B00/xxx" \
  --method POST \
  --severity warning
```

| Parameter      | Default   | Description                                     |
|----------------|-----------|-------------------------------------------------|
| `--method`     | `POST`    | HTTP method (`POST` or `GET`)                   |
| `--severity`   | `warning` | Minimum severity: `info`, `warning`, `error`, `critical` |

### List channels

```bash
aether webhook list
```

### Test a channel

```bash
aether webhook test slack-alerts
```

### View the retry queue

Failed webhook deliveries are placed in a retry queue with exponential backoff:

```bash
aether webhook queue
```

### Force-retry all queued webhooks

```bash
aether webhook flush
```

### Remove a channel

```bash
aether webhook remove slack-alerts
```

---

## 🔁 Background Reconciliation Loop

The `orchestrate watch` command runs a continuous health monitoring loop with
automatic healing:

```bash
aether orchestrate watch --interval 30
```

This runs in the foreground and performs the following every interval:

1. **Health check** all registered workloads
2. **Record** health observations (uptime, latency, restart count)
3. **Evaluate alert rules** against current system metrics (SLA uptimes, restart
   counts, drift, policy violations, secret expiry) with per-rule cooldown
4. **Circuit breaker** detection -- if a workload fails repeatedly, the circuit
   opens and further attempts are paused
5. **Auto-remediation** via rolling updates when health degrades

### Register workloads for orchestration

```bash
aether orchestrate register api-service --runtime kubernetes
aether orchestrate register web-frontend --runtime podman
```

### View orchestration status

```bash
aether orchestrate status
```

### Rolling update through the orchestrator

```bash
aether orchestrate rolling-update api-service --replicas 3
```

### Reset a tripped circuit breaker

```bash
aether orchestrate reset-circuit api-service
```

---

## 💰 Cost Estimation Across Providers

Estimate what your workload would cost across major cloud providers:

```bash
aether cost
```

```
Cost Estimation for 'hello-web' (2 CPU, 4Gi RAM, 20Gi storage)

  ┌─────────────────┬──────────────┬──────────────┬──────────────┐
  │ Provider        │ Monthly      │ Hourly       │ Tier         │
  ├─────────────────┼──────────────┼──────────────┼──────────────┤
  │ AWS             │ $52.40       │ $0.072       │ t3.large     │
  │ Azure           │ $48.90       │ $0.067       │ B2s          │
  │ GCP             │ $46.72       │ $0.064       │ e2-standard-2│
  │ DigitalOcean    │ $24.00       │ $0.033       │ s-2vcpu-4gb  │
  │ Linode          │ $20.00       │ $0.027       │ g6-standard-2│
  └─────────────────┴──────────────┴──────────────┴──────────────┘
```

Filter by provider:

```bash
aether cost --provider aws
aether cost --provider gcp
aether cost --provider all      # default
```

---

## 🤖 AI-Powered Recommendations

Aether includes an AI recommendation engine that analyzes your workload
spec and provides actionable advice.

### Runtime recommendation

```bash
aether recommend
```

Scores each runtime (0--100) based on resource requirements, networking needs,
persistence, GPU, and cost optimization.

### Workload profiling

```bash
aether profile --name hello-web
```

Analyzes a deployed workload and suggests optimization opportunities for
resource usage, scaling, and cost.

### Log analysis

```bash
aether analyze-logs hello-web
```

Scans workload logs for anomalies, error patterns, and recurring issues.

### Migration advice

```bash
aether migration-advice hello-web kube
```

Provides tailored guidance for migrating a specific workload to a target
runtime, including:

- Risk assessment
- Recommended strategy (immediate, blue-green, rolling)
- Pre-migration checklist
- Estimated migration time

### Scaling advice

```bash
aether scaling-advice
```

Predicts scaling needs based on historical health data and resource utilization
patterns.

### Runtime affinity

```bash
# Get recommendations for a workload class
aether affinity recommend web-service

# View the compatibility matrix
aether affinity matrix

# View learning statistics
aether affinity stats
```

Supported workload classes: `web-service`, `api-backend`, `database`, `cache`,
`batch-job`, `ml-training`, `worker`, `microservice`.

---

## 🎯 Next Steps

You have now explored the full depth of Aether's advanced features. Here
are the reference documents:

| Document                                                                 | Description                                     |
|--------------------------------------------------------------------------|------------------------------------------------|
| [CLI Reference](../guides/cli/CLI-Reference.md)                         | Complete reference for all commands              |
| [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md)       | Step-by-step migration procedures               |

### Features to explore on your own

```bash
# Manage deployment environments
aether env create staging --tier staging
aether env promote hello-web development staging
aether env parity staging production

# SLA compliance monitoring
aether sla add hello-web --tier high-availability
aether sla check hello-web --uptime 99.95 --latency 20

# Workload dependency management
aether deps add web-frontend api-service
aether deps show
aether deps order
aether deps impact api-service

# Scheduling and placement optimization
aether schedule place hello-web --cpu 4 --memory 8192 --strategy balanced
aether schedule utilization
aether schedule optimize

# Audit trail
aether audit --last 50 --workload hello-web

# Event stream
aether events --last 20 --severity warning

# Templates
aether template --list
aether template rest-api --workload-name my-api --registry ghcr.io/org
```

---

> 📚 **Full documentation:** [CLI Reference](../guides/cli/CLI-Reference.md)
> 🏷 **License:** Proprietary HyperSDK
