# 🌐 REST API Reference

> Complete reference for the aether REST API -- 43 endpoints across workloads, AI, security, and operations.

---

## 📑 Table of Contents

- [Server Setup](#-server-setup)
- [Authentication](#-authentication)
- [Response Format](#-response-format)
- [Dashboard and Health](#-dashboard-and-health)
- [Workload Management](#-workload-management)
- [Build and Validate](#-build-and-validate)
- [Migration](#-migration)
- [Cost Estimation](#-cost-estimation)
- [Backups](#-backups)
- [Plugins](#-plugins)
- [Health Monitoring](#-health-monitoring)
- [Compose](#-compose)
- [Secrets](#-secrets)
- [AI and Intelligence](#-ai-and-intelligence)
- [Drift Detection](#-drift-detection)
- [Policy Enforcement](#-policy-enforcement)
- [Dependencies](#-dependencies)
- [Audit Trail](#-audit-trail)
- [Templates](#-templates)
- [SLA Compliance](#-sla-compliance)
- [Events](#-events)
- [Environments](#-environments)
- [Scheduler](#-scheduler)
- [Orchestrator](#-orchestrator)
- [Affinity](#-affinity)
- [Metrics](#-metrics)
- [Endpoint Summary Table](#-endpoint-summary-table)

---

## 🚀 Server Setup

Start the API server and embedded web dashboard:

```bash
# Default: localhost:8080
aether serve

# Custom host and port
aether serve --host 0.0.0.0 --port 3000
```

| Flag | Default | Description |
|---|---|---|
| `--host` | `127.0.0.1` | Server bind address |
| `--port` / `-p` | `8080` | Server port |

On startup, the server prints:

```
🌐 Starting API server on http://127.0.0.1:8080
📊 Dashboard: http://127.0.0.1:8080
📋 API Health: http://127.0.0.1:8080/health
```

The server uses Axum with Tokio for async request handling and loads workload state from `~/.aether/state.json` into a shared `Arc<RwLock<StateStore>>`.

---

## 🔑 Authentication

The API server currently has **no built-in authentication**. It binds to `127.0.0.1` by default (localhost only).

### Production Recommendations

| Recommendation | Implementation |
|---|---|
| Reverse proxy with auth | Nginx, Caddy, or Traefik with OAuth2, mTLS, or API keys |
| Bind to localhost only | Use the default `--host 127.0.0.1` |
| Network isolation | Firewall rules, VPN, or service mesh |
| API gateway | Kong, Ambassador, or cloud-native gateway |
| TLS termination | Let the reverse proxy handle HTTPS |

---

## 📦 Response Format

All API responses use the `ApiResponse<T>` wrapper:

**Success:**

```json
{
  "success": true,
  "data": <response-payload>,
  "error": null
}
```

**Error:**

```json
{
  "success": false,
  "data": null,
  "error": "Human-readable error message"
}
```

### HTTP Status Codes

| Code | Meaning | When Used |
|---|---|---|
| `200` | OK | Successful read or mutation |
| `201` | Created | New resource created (workload, backup, dependency) |
| `400` | Bad Request | Invalid input (bad runtime name, invalid YAML) |
| `404` | Not Found | Workload, secret, or SLA target not found |
| `500` | Internal Server Error | Runtime failure, state persistence error |

---

## 🏠 Dashboard and Health

### GET `/` -- Web Dashboard

Serves the embedded HTML web dashboard with real-time workload management UI.

```bash
curl http://localhost:8080/
# Returns: HTML document
```

---

### GET `/health` -- Health Check

Lightweight health check endpoint for load balancers and monitoring.

```bash
curl http://localhost:8080/health
```

**Response:**

```json
{
  "success": true,
  "data": {
    "status": "ok",
    "version": "0.3.0"
  },
  "error": null
}
```

---

## 📋 Workload Management

### GET `/api/workloads` -- List All Workloads

```bash
curl http://localhost:8080/api/workloads
```

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "name": "my-app",
      "runtime": "Podman",
      "image": "my-app:latest",
      "status": "deployed (podman)",
      "created_at": "2026-01-15T10:00:00Z"
    }
  ],
  "error": null
}
```

---

### POST `/api/workloads` -- Create and Deploy Workload

```bash
curl -X POST http://localhost:8080/api/workloads \
  -H "Content-Type: application/json" \
  -d '{
    "spec": {
      "apiVersion": "aether/v1",
      "kind": "Workload",
      "metadata": { "name": "my-app", "owner": "team", "project": "demo" },
      "build": { "context": ".", "dockerfile": "Dockerfile", "registry": "ghcr.io/org" },
      "requirements": { "cpu": "2", "memory": "4Gi", "storage": "10Gi" },
      "runtime": { "preferred": "auto", "allow": ["container", "kube"] }
    },
    "runtime": "podman"
  }'
```

| Field | Required | Description |
|---|---|---|
| `spec` | Yes | Full workload spec as JSON object |
| `runtime` | No | Override runtime selection. When omitted, the AI decision engine selects automatically. |

**Response (201):**

```json
{ "success": true, "data": "Workload my-app created", "error": null }
```

---

### GET `/api/workloads/:name` -- Get Workload Details

```bash
curl http://localhost:8080/api/workloads/my-app
```

**Response:**

```json
{
  "success": true,
  "data": {
    "name": "my-app",
    "runtime": "Podman",
    "image": "my-app:latest",
    "status": "deployed (podman)",
    "created_at": "2026-01-15T10:00:00Z"
  },
  "error": null
}
```

---

### GET `/api/workloads/:name/logs` -- Get Workload Logs

```bash
curl http://localhost:8080/api/workloads/my-app/logs
```

**Response:** `"data"` contains the log output as a string.

---

### POST `/api/workloads/:name/start` -- Start a Workload

Rebuilds and runs a workload from its stored spec. Guards against concurrent deletion.

```bash
curl -X POST http://localhost:8080/api/workloads/my-app/start
```

**Response:**

```json
{ "success": true, "data": "Workload my-app started", "error": null }
```

---

### POST `/api/workloads/:name/stop` -- Stop a Workload

```bash
curl -X POST http://localhost:8080/api/workloads/my-app/stop
```

**Response:**

```json
{ "success": true, "data": "Workload my-app stopped", "error": null }
```

---

### DELETE `/api/workloads/:name` -- Delete a Workload

```bash
curl -X DELETE http://localhost:8080/api/workloads/my-app
```

**Response:**

```json
{ "success": true, "data": "Workload my-app deleted", "error": null }
```

---

## 🔨 Build and Validate

### POST `/api/workloads/:name/build` -- Trigger a Build

Builds an image for an existing workload using its stored spec and runtime.

```bash
curl -X POST http://localhost:8080/api/workloads/my-app/build
```

**Response:**

```json
{
  "success": true,
  "data": {
    "image_name": "my-app",
    "image_tag": "latest",
    "full_name": "my-app:latest",
    "runtime": "podman"
  },
  "error": null
}
```

---

### POST `/api/validate` -- Validate a Workload Spec

Validates YAML without deploying.

```bash
curl -X POST http://localhost:8080/api/validate \
  -H "Content-Type: application/json" \
  -d '{"yaml": "apiVersion: aether/v1\nkind: Workload\n..."}'
```

**Response (valid):**

```json
{
  "success": true,
  "data": { "valid": true, "workload_name": "my-app", "errors": [] },
  "error": null
}
```

**Response (invalid):**

```json
{
  "success": true,
  "data": { "valid": false, "workload_name": null, "errors": ["YAML parse error: ..."] },
  "error": null
}
```

---

## 🔄 Migration

### POST `/api/workloads/:name/migrate` -- Migrate a Workload

Migrate a deployed workload to a different runtime with zero-downtime strategies.

```bash
curl -X POST http://localhost:8080/api/workloads/my-app/migrate \
  -H "Content-Type: application/json" \
  -d '{"target_runtime": "kubernetes", "strategy": "blue-green"}'
```

| Field | Required | Default | Description |
|---|---|---|---|
| `target_runtime` | Yes | -- | Target runtime: `podman`, `kubernetes`, `kubevirt`, `metal3` |
| `strategy` | No | `"blue-green"` | Strategy: `immediate`, `blue-green`, `rolling` |

**Response (success):**

```json
{
  "success": true,
  "data": "Workload my-app migrated from podman to kubernetes (strategy: blue-green, duration: 12.3s)",
  "error": null
}
```

**Response (failure with rollback):**

```json
{
  "success": false,
  "data": null,
  "error": "Migration failed: Target instance failed health check (rollback performed)"
}
```

---

## 💰 Cost Estimation

### POST `/api/cost` -- Estimate Workload Costs

```bash
curl -X POST http://localhost:8080/api/cost \
  -H "Content-Type: application/json" \
  -d @workload.json
```

**Request body:** Full workload spec as JSON.

**Response:** Array of `CostEstimate` objects per cloud provider (AWS, Azure, GCP, DigitalOcean, Linode).

---

## 💾 Backups

### GET `/api/backups` -- List Backups

```bash
curl http://localhost:8080/api/backups
```

**Response:** Array of backup file paths.

---

### POST `/api/backups` -- Create a Backup

```bash
curl -X POST http://localhost:8080/api/backups \
  -H "Content-Type: application/json" \
  -d '{"name": "pre-migration", "description": "Before K8s migration"}'
```

| Field | Required | Description |
|---|---|---|
| `name` | No | Backup name (auto-generated if omitted) |
| `description` | No | Human-readable description |

**Response (201):** Backup file path.

---

## 🔌 Plugins

### GET `/api/plugins` -- List Registered Plugins

```bash
curl http://localhost:8080/api/plugins
```

**Response:** Array of `PluginManifest` objects with `name`, `version`, `runtime_kind`, `command`, `capabilities`.

---

### POST `/api/plugins/discover` -- Discover Plugins

Scans `~/.aether/plugins/` for manifest files and updates the registry.

```bash
curl -X POST http://localhost:8080/api/plugins/discover
```

**Response:**

```json
{
  "success": true,
  "data": { "discovered": 2, "total": 3 },
  "error": null
}
```

---

## 🏥 Health Monitoring

### GET `/api/health/:workload` -- Health Summary

```bash
curl http://localhost:8080/api/health/my-app
```

**Response:**

```json
{
  "success": true,
  "data": {
    "workload": "my-app",
    "total_checks": 156,
    "ready_checks": 152,
    "uptime_percent": 97.44,
    "last_restart_count": 2,
    "last_state": "running"
  },
  "error": null
}
```

---

## 🎼 Compose

### POST `/api/compose/validate` -- Validate Compose Spec

Validates a compose YAML for structural correctness, dependency cycles, and missing references.

```bash
curl -X POST http://localhost:8080/api/compose/validate \
  -H "Content-Type: text/plain" \
  -d @aether-compose.yaml
```

**Response (valid):**

```json
{
  "success": true,
  "data": { "valid": true, "workload_count": 3, "deploy_order": ["database", "api", "web"] },
  "error": null
}
```

**Response (invalid):**

```json
{ "success": false, "data": null, "error": "circular dependency detected in compose workloads" }
```

---

## 🔐 Secrets

### GET `/api/secrets` -- List Secrets

```bash
curl http://localhost:8080/api/secrets
```

**Response:** Array of `SecretSummary` objects (name, namespace, key count, timestamps, rotation status). **Values are never exposed.**

---

### GET `/api/secrets/:name` -- Get Secret Metadata

```bash
curl http://localhost:8080/api/secrets/db-creds
```

**Response:**

```json
{
  "success": true,
  "data": {
    "name": "db-creds",
    "namespace": "production",
    "key_count": 2,
    "keys": ["username", "password"],
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-15T00:00:00Z",
    "needs_rotation": false,
    "rotation_policy": {
      "interval_days": 90,
      "max_age_days": 365,
      "notify_before_days": 14
    }
  },
  "error": null
}
```

---

### DELETE `/api/secrets/:name` -- Delete a Secret

```bash
curl -X DELETE http://localhost:8080/api/secrets/old-creds
```

**Response:**

```json
{ "success": true, "data": "Secret 'old-creds' deleted", "error": null }
```

---

## 🧠 AI and Intelligence

### POST `/api/ai/recommend` -- Runtime Recommendation

Scores the workload spec against all runtimes using the AI scoring engine.

```bash
curl -X POST http://localhost:8080/api/ai/recommend \
  -H "Content-Type: application/json" \
  -d @workload.json
```

---

### GET `/api/ai/profile/:name` -- Workload Profile

Profiles a deployed workload for resource waste and optimization opportunities.

```bash
curl http://localhost:8080/api/ai/profile/my-app
```

---

### GET `/api/ai/analyze/:name` -- Log Analysis

Analyzes workload logs for anomalies, error patterns, and trends.

```bash
curl http://localhost:8080/api/ai/analyze/my-app
```

---

### GET `/api/ai/migration-advice/:name/:target` -- Migration Advice

Provides risk assessment, strategy recommendation, timing advice, and canary configuration for a migration path.

```bash
curl http://localhost:8080/api/ai/migration-advice/my-app/kubernetes
```

**Response:**

```json
{
  "success": true,
  "data": {
    "workload_name": "my-app",
    "source_runtime": "Podman",
    "target_runtime": "Kubernetes",
    "recommended_strategy": "BlueGreen",
    "estimated_downtime_secs": 30,
    "risk_level": "Medium",
    "reasons": ["Better scaling capabilities"],
    "warnings": ["Requires PVC migration"],
    "timing": {
      "recommendation": "Off-peak hours",
      "preferred_window": "02:00-06:00 UTC",
      "avoid_times": ["Peak hours"]
    },
    "canary_config": {
      "steps": [10, 25, 50, 100],
      "step_interval_secs": 300,
      "error_threshold": 0.05,
      "latency_threshold_pct": 10.0,
      "min_observation_secs": 120
    }
  },
  "error": null
}
```

---

### GET `/api/ai/scaling-advice` -- Scaling Recommendations

Predictive scaling advice with forecast trend, cost impact, and confidence score.

```bash
curl http://localhost:8080/api/ai/scaling-advice
```

**Response includes:** action, current/recommended replicas, reason, confidence, forecast (trend, predicted value, bounds, horizon), and cost impact (current/projected hourly, delta hourly/monthly).

---

## 🔍 Drift Detection

### GET `/api/drift/:name` -- Check Configuration Drift

Compares a workload's spec against its live state to detect drift.

```bash
curl http://localhost:8080/api/drift/my-app
```

---

## 🛡️ Policy Enforcement

### POST `/api/policy/check` -- Policy Check

Evaluate a workload spec against a policy set.

```bash
curl -X POST http://localhost:8080/api/policy/check \
  -H "Content-Type: application/json" \
  -d '{"spec": { ... }, "policy_set": "production"}'
```

| Field | Required | Default | Description |
|---|---|---|---|
| `spec` | Yes | -- | Workload spec as JSON |
| `policy_set` | No | `"production"` | `"production"` or `"development"` |

---

## 🔗 Dependencies

### GET `/api/dependencies` -- Show Dependency Graph

```bash
curl http://localhost:8080/api/dependencies
```

**Response:** Graph stats, startup order, and validation issues.

---

### POST `/api/dependencies` -- Add a Dependency

```bash
curl -X POST http://localhost:8080/api/dependencies \
  -H "Content-Type: application/json" \
  -d '{"workload": "frontend", "dependency": "backend"}'
```

**Response (201):** `"Dependency added: frontend -> backend"`

---

## 📜 Audit Trail

### GET `/api/audit` -- List Audit Events

```bash
curl http://localhost:8080/api/audit
```

**Response:** Summary statistics and the 20 most recent audit events.

---

## 📝 Templates

### GET `/api/templates` -- List Available Templates

```bash
curl http://localhost:8080/api/templates
```

Available templates: `web-app`, `rest-api`, `database`, `cache`, `worker`, `cron-job`, `ml-training`, `microservice`.

---

### POST `/api/templates/:name` -- Generate from Template

```bash
curl -X POST http://localhost:8080/api/templates/web-app \
  -H "Content-Type: application/json" \
  -d '{"workload_name": "my-site", "port": 8080, "replicas": 3}'
```

| Field | Required | Description |
|---|---|---|
| `workload_name` | No | Name for the generated workload |
| `owner` | No | Owner label |
| `project` | No | Project label |
| `registry` | No | Container registry |
| `cpu` | No | CPU requirement |
| `memory` | No | Memory requirement |
| `port` | No | Application port |
| `replicas` | No | Replica count |

---

## 📊 SLA Compliance

### GET `/api/sla/:workload` -- Check SLA

```bash
curl http://localhost:8080/api/sla/my-app
```

Returns the SLA target for the specified workload, or 404 if not configured.

---

## 📢 Events

### GET `/api/events` -- List Recent Events

```bash
curl http://localhost:8080/api/events
```

**Response:** Last 50 events with timestamps, severity, and details.

---

### GET `/api/events/summary` -- Event Summary

```bash
curl http://localhost:8080/api/events/summary
```

**Response:** Aggregated event counts by type and severity.

---

## 🌍 Environments

### GET `/api/environments` -- List Environments

```bash
curl http://localhost:8080/api/environments
```

**Response:** All configured environments (development, staging, production).

---

## ⚖️ Scheduler

### GET `/api/scheduler/utilization` -- Runtime Utilization

```bash
curl http://localhost:8080/api/scheduler/utilization
```

**Response:** CPU and memory utilization per runtime.

---

### GET `/api/scheduler/optimize` -- Optimization Suggestions

```bash
curl http://localhost:8080/api/scheduler/optimize
```

**Response:** Suggestions for workload placement improvements.

---

## 🎭 Orchestrator

### GET `/api/orchestrator/status` -- Managed Workload Statuses

```bash
curl http://localhost:8080/api/orchestrator/status
```

**Response:** Health status of all orchestrator-managed workloads.

---

### GET `/api/orchestrator/summary` -- Health Summary

```bash
curl http://localhost:8080/api/orchestrator/summary
```

**Response:** Aggregated health metrics across all managed workloads.

---

## 🧲 Affinity

### GET `/api/affinity/:class` -- Runtime Affinity Recommendation

```bash
curl http://localhost:8080/api/affinity/web-service
```

**Valid classes:** `web-service`, `api-backend`, `database`, `cache`, `batch-job`, `ml-training`, `worker`, `microservice`.

**Response:** Per-runtime affinity scores for the given workload class.

---

## 📈 Metrics

### GET `/api/metrics` -- Prometheus Metrics

```bash
curl http://localhost:8080/api/metrics
```

**Content-Type:** `text/plain; charset=utf-8`

Returns Prometheus-format metrics text for scraping by Prometheus, Grafana Agent, or compatible collectors.

---

## 📚 Endpoint Summary Table

| Method | Path | Description |
|---|---|---|
| **Dashboard** | | |
| GET | `/` | Web dashboard (HTML) |
| GET | `/health` | Health check |
| **Workloads** | | |
| GET | `/api/workloads` | List all workloads |
| POST | `/api/workloads` | Create and deploy workload |
| GET | `/api/workloads/:name` | Get workload details |
| DELETE | `/api/workloads/:name` | Delete workload |
| GET | `/api/workloads/:name/logs` | Get logs |
| POST | `/api/workloads/:name/start` | Start workload |
| POST | `/api/workloads/:name/stop` | Stop workload |
| POST | `/api/workloads/:name/migrate` | Migrate workload |
| POST | `/api/workloads/:name/build` | Trigger build |
| **Validation** | | |
| POST | `/api/validate` | Validate workload spec YAML |
| POST | `/api/compose/validate` | Validate compose file |
| **Cost** | | |
| POST | `/api/cost` | Estimate workload costs |
| **Backups** | | |
| GET | `/api/backups` | List backups |
| POST | `/api/backups` | Create backup |
| **Plugins** | | |
| GET | `/api/plugins` | List plugins |
| POST | `/api/plugins/discover` | Discover plugins |
| **Health** | | |
| GET | `/api/health/:workload` | Health summary |
| **Secrets** | | |
| GET | `/api/secrets` | List secrets |
| GET | `/api/secrets/:name` | Secret metadata |
| DELETE | `/api/secrets/:name` | Delete secret |
| **AI** | | |
| POST | `/api/ai/recommend` | Runtime recommendation |
| GET | `/api/ai/profile/:name` | Workload profile |
| GET | `/api/ai/analyze/:name` | Log analysis |
| GET | `/api/ai/migration-advice/:name/:target` | Migration advice |
| GET | `/api/ai/scaling-advice` | Scaling advice |
| **Operations** | | |
| GET | `/api/drift/:name` | Drift check |
| POST | `/api/policy/check` | Policy check |
| GET | `/api/dependencies` | Show dependencies |
| POST | `/api/dependencies` | Add dependency |
| GET | `/api/audit` | Audit events |
| GET | `/api/templates` | List templates |
| POST | `/api/templates/:name` | Generate from template |
| GET | `/api/sla/:workload` | SLA check |
| GET | `/api/events` | List events |
| GET | `/api/events/summary` | Event summary |
| GET | `/api/environments` | List environments |
| GET | `/api/scheduler/utilization` | Scheduler utilization |
| GET | `/api/scheduler/optimize` | Optimizer suggestions |
| GET | `/api/orchestrator/status` | Orchestrator status |
| GET | `/api/orchestrator/summary` | Orchestrator summary |
| GET | `/api/affinity/:class` | Affinity recommendation |
| GET | `/api/metrics` | Prometheus metrics (text/plain) |
