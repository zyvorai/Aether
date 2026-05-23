# 🏥 Health Monitoring & Uptime Tracking

> Track workload health over time with historical records, uptime calculations, and timeline views.

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Health Records](#-health-records)
- [Uptime Calculation](#-uptime-calculation)
- [Timeline View](#-timeline-view)
- [Summary View](#-summary-view)
- [Integration with Status Command](#-integration-with-status-command)
- [Background Health Checks](#-background-health-checks)
- [Health in the TUI Dashboard](#-health-in-the-tui-dashboard)
- [REST API Endpoint](#-rest-api-endpoint)
- [Data Storage and Retention](#-data-storage-and-retention)
- [Cross-References](#-cross-references)

---

## 🔎 Overview

aether's health monitoring system records periodic health check observations for each workload, building a historical timeline that enables:

- 📈 **Uptime percentage** calculations (ratio of ready checks to total checks)
- 🔄 **Restart tracking** across time
- 🕐 **Health timeline** showing state transitions
- 📋 **Summary statistics** per workload
- 🗂️ **Bounded storage** with automatic pruning of old records

Health data is stored in `~/.aether/health.json` and is updated whenever you run status checks, background watches, or orchestration health checks.

---

## 📊 Health Records

Each health check produces a `HealthRecord` with the following fields:

| Field | Type | Description |
|---|---|---|
| `timestamp` | string (RFC 3339) | When the health check occurred (e.g. `2026-01-15T10:00:00Z`) |
| `workload` | string | Name of the workload |
| `runtime` | RuntimeKind | Runtime hosting the workload: `podman`, `kubernetes`, `kubevirt`, `metal3` |
| `state` | InstanceState | Instance state at check time: `pending`, `running`, `stopped`, `failed`, `unknown` |
| `ready` | bool | Whether the workload was healthy/ready at check time |
| `restart_count` | u32 | Cumulative restart count reported by the runtime |
| `latency_ms` | Option\<f64\> | Health check probe latency in milliseconds (null if not measured) |

### Example Record (JSON)

```json
{
  "timestamp": "2026-01-15T10:05:00Z",
  "workload": "api-service",
  "runtime": "kubernetes",
  "state": "running",
  "ready": true,
  "restart_count": 0,
  "latency_ms": 12.5
}
```

### Instance States

| State | Icon | Description |
|---|---|---|
| `running` | 🟢 | Workload is running normally |
| `pending` | 🟡 | Workload is starting or transitioning |
| `stopped` | ⚪ | Workload was intentionally stopped |
| `failed` | 🔴 | Workload has failed |
| `unknown` | ❓ | State cannot be determined |

---

## 📈 Uptime Calculation

Uptime is calculated as the percentage of health checks where `ready == true`:

```
uptime_percent = (ready_checks / total_checks) * 100.0
```

### Examples

| Scenario | Ready Checks | Total Checks | Uptime |
|---|---|---|---|
| Always healthy | 100 | 100 | **100.00%** |
| 3 of 4 healthy | 3 | 4 | **75.00%** |
| Two-thirds healthy | 2 | 3 | **66.67%** |
| Never healthy | 0 | 50 | **0.00%** |
| No records | 0 | 0 | **0.00%** |

### Key Behaviors

- Uptime is calculated **per workload** -- records from other workloads are excluded
- If no records exist for a workload, uptime returns `0.0`
- The calculation spans **all retained records** (up to `max_records`)
- Multiple workloads are fully isolated: good uptime on "web" is not affected by "api" failures

---

## 🕐 Timeline View

The timeline view shows the most recent health records for a workload in chronological order (oldest first).

```bash
aether health api-service                  # Default: last 20 records
aether health api-service --last 10        # Last 10 records
aether health api-service --last 50        # Last 50 records
```

### Example Timeline Output

```
╭────────────────────────┬───────────┬─────────┬───────┬──────────┬────────────╮
│ Timestamp              │ Runtime   │ State   │ Ready │ Restarts │ Latency    │
├────────────────────────┼───────────┼─────────┼───────┼──────────┼────────────┤
│ 2026-01-15T10:00:00Z   │ ☸️ kube   │ running │ ✅    │ 0        │ 11.2ms     │
│ 2026-01-15T10:05:00Z   │ ☸️ kube   │ running │ ✅    │ 0        │ 12.5ms     │
│ 2026-01-15T10:10:00Z   │ ☸️ kube   │ failed  │ ❌    │ 1        │ --         │
│ 2026-01-15T10:15:00Z   │ ☸️ kube   │ running │ ✅    │ 1        │ 15.0ms     │
╰────────────────────────┴───────────┴─────────┴───────┴──────────┴────────────╯
  Uptime: 75.00%  |  3 / 4 checks ready  |  1 restart(s)
```

### Timeline Behavior

| Situation | Behavior |
|---|---|
| `--last N` with fewer than N records | Returns all available records |
| `--last 0` | Returns empty timeline |
| No records for workload | Returns empty timeline |
| Records from other workloads | Filtered out (only target workload shown) |

---

## 📋 Summary View

Get a high-level summary of a workload's health without individual records.

```bash
aether health api-service --summary
```

### Summary Fields

| Field | Type | Description |
|---|---|---|
| `workload` | string | Workload name |
| `total_checks` | usize | Total number of health checks recorded |
| `ready_checks` | usize | Number of checks where the workload was ready |
| `uptime_percent` | f64 | Uptime percentage (0.0 -- 100.0) |
| `last_restart_count` | u32 | Most recent restart count from the runtime |
| `last_state` | string | String representation of the most recent instance state |

### Example Summary Output

```
╭────────────────────┬────────────────╮
│ Property           │ Value          │
├────────────────────┼────────────────┤
│ Workload           │ api-service    │
│ Total Checks       │ 156            │
│ Ready Checks       │ 152            │
│ Uptime             │ 97.44%         │
│ Last Restart Count │ 2              │
│ Last State         │ running        │
╰────────────────────┴────────────────╯
```

### Edge Cases

| Scenario | total_checks | ready_checks | uptime_percent | last_state |
|---|---|---|---|---|
| No records | 0 | 0 | 0.0 | `"unknown"` |
| All ready | N | N | 100.0 | `"running"` |
| Never ready | N | 0 | 0.0 | `"failed"` |

---

## 🐳 Podman Native Health Checks

When deploying to Podman, Aether automatically maps workload health probes to native Podman health check flags. This enables container-level health monitoring without external tooling.

### Probe Type Mapping

| Probe Type | Podman `--health-cmd` |
|---|---|
| `httpGet` | `curl -sf http://localhost:{port}{path} \|\| exit 1` |
| `tcpSocket` | `bash -c '</dev/tcp/localhost/{port}' \|\| exit 1` |
| `exec` | The command array joined as a single string |

### Configuration

Health check timing is derived from the workload spec's `health.liveness` probe:

| Spec Field | Podman Flag | Description |
|---|---|---|
| `period_seconds` | `--health-interval` | Time between health checks |
| `initial_delay_seconds` | `--health-start-period` | Grace period before first check |

### Restart Policy

All Podman containers are started with `--restart on-failure:3`, providing automatic restart resilience for up to 3 consecutive failures.

### Health-Aware Status

The `aether status` command queries Podman's health subsystem and returns enriched status information:

| Field | Source | Description |
|---|---|---|
| `state` | Container state | `running`, `stopped`, `pending`, `unknown` |
| `ready` | Health check result | `true` if healthy or no health check configured |
| `message` | Health status | `"Health check failing"` or `"Health check starting"` when applicable |
| `restart_count` | `RestartCount` | Number of times the container has been restarted |

### Example

```yaml
health:
  liveness:
    httpGet:
      path: /health
      port: 8080
    initialDelaySeconds: 10
    periodSeconds: 5
```

This translates to:

```bash
podman run \
  --health-cmd "curl -sf http://localhost:5090/health || exit 1" \
  --health-interval 5s \
  --health-start-period 10s \
  --restart on-failure:3 \
  ...
```

Status output will then show:

```
State:     running
Ready:     true (healthy)
Restarts:  0
```

---

## 🔄 Integration with Status Command

The `aether status <name>` command automatically records a health check when it queries a workload's status. This means every status check contributes to the health history.

```bash
# This queries status AND records a health check
aether status my-app

# View the accumulated history
aether health my-app
```

The recorded health check captures:

| Data | Source |
|---|---|
| `state` | Current `InstanceState` from the runtime |
| `ready` | The `ready` flag from the status response |
| `restart_count` | The `restart_count` from the runtime |
| `latency_ms` | Probe response time (when available) |
| `timestamp` | Current time in RFC 3339 |
| `runtime` | The runtime kind of the workload |

---

## 👁️ Background Health Checks

### Continuous Monitoring

Use `aether orchestrate watch` to run continuous health monitoring at a configurable interval.

```bash
# Check every 30 seconds (default)
aether orchestrate watch

# Check every 10 seconds
aether orchestrate watch --interval 10

# Check every 5 minutes
aether orchestrate watch --interval 300
```

Each watch cycle:

1. Queries status of **all registered workloads** across all runtimes
2. Records a `HealthRecord` for each workload
3. Saves the updated health history to disk
4. Evaluates **alert rules** against current system metrics (SLA uptimes, restart counts, drift, policy violations, secret expiry)
5. Triggers notifications for fired alerts via configured webhook channels
6. Triggers circuit breaker alerts if workloads fail repeatedly

### One-Shot Health Check

Run a single round of health checks against all live runtimes without entering a continuous loop:

```bash
aether orchestrate health-check
```

### Other Orchestration Commands

```bash
aether orchestrate register my-app --runtime kubernetes  # Register for monitoring
aether orchestrate status                                 # Show all health statuses
aether orchestrate summary                                # Health summary
aether orchestrate reset-circuit my-app                   # Reset circuit breaker
```

---

## 🖥️ Health in the TUI Dashboard

Launch the TUI with `aether tui` to see a real-time dashboard with health information.

### Dashboard Layout

| Panel | Content |
|---|---|
| **Workload List** | All deployed workloads with runtime icon (🐳 ☸️ 🖥️ 🖧), name, and status |
| **Status Detail** | Selected workload's state, ready flag, restart count |
| **Resource Panel** | CPU, memory, and health indicators for the selected workload |
| **Search Bar** | Filter workloads by name (toggle with `/`) |

### Health Indicators

| Indicator | Meaning |
|---|---|
| 🟢 **Healthy** | Workload is running and ready |
| 🟡 **Pending** | Workload is starting or transitioning |
| 🔴 **Unhealthy** | Workload is failed or not ready |

### TUI Keyboard Shortcuts

| Key | Action |
|---|---|
| `q` / `Ctrl+C` | Quit the TUI |
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `Enter` | View logs for selected workload |
| `Esc` | Return to dashboard from logs view |
| `r` | Refresh workload statuses |
| `/` | Toggle search/filter mode |

### Auto-Refresh

The TUI periodically refreshes workload statuses. Each refresh reuses runtime clients grouped by runtime kind for efficiency (one client per runtime, not per workload).

---

## 🌐 REST API Endpoint

### GET `/api/health/:workload` -- Health Summary

Retrieve the health summary for a specific workload.

```bash
curl http://localhost:5090/api/health/api-service
```

**Response (workload with history):**

```json
{
  "success": true,
  "data": {
    "workload": "api-service",
    "total_checks": 156,
    "ready_checks": 152,
    "uptime_percent": 97.44,
    "last_restart_count": 2,
    "last_state": "running"
  },
  "error": null
}
```

**Response (workload with no records):**

```json
{
  "success": true,
  "data": {
    "workload": "unknown-app",
    "total_checks": 0,
    "ready_checks": 0,
    "uptime_percent": 0.0,
    "last_restart_count": 0,
    "last_state": "unknown"
  },
  "error": null
}
```

> **Note:** The endpoint always returns a `200 OK` with a valid summary, even when no records exist. The `total_checks: 0` field indicates no health data has been collected yet.

---

## 💾 Data Storage and Retention

### Storage Location

```
~/.aether/health.json
```

### Retention Policy

The health history uses a **bounded ring buffer** that prunes the oldest records when `max_records` is reached.

| Setting | Default | Description |
|---|---|---|
| `max_records` | 1000 | Maximum number of records retained **across all workloads** |

### Pruning Behavior

When a new record is added and the buffer is at capacity:

1. Oldest records are drained to make room (drain count = `current_length - max_records + 1`)
2. The new record is appended at the end
3. The buffer never exceeds `max_records`

### Persistence Behavior

| Scenario | Behavior |
|---|---|
| Save after health check | Records written to disk as JSON |
| Load from existing file | Deserializes with full fidelity |
| Load from missing file | Returns empty history with `max_records: 1000` (no error) |
| Default path | `~/.aether/health.json` (auto-detected from `$HOME`) |

### Storage Format

```json
{
  "records": [
    {
      "timestamp": "2026-01-15T10:00:00Z",
      "workload": "api-service",
      "runtime": "kubernetes",
      "state": "running",
      "ready": true,
      "restart_count": 0,
      "latency_ms": 12.5
    }
  ],
  "max_records": 1000
}
```

---

## 🚨 Alert Rule Evaluation

The `orchestrate watch` loop evaluates configured alert rules against live system metrics on every monitoring cycle. When a rule's condition is met, an event is emitted and notifications are sent via configured channels.

### System Metrics

Each watch cycle collects the following metrics for evaluation:

| Metric | Source | Description |
|---|---|---|
| `sla_uptimes` | Health history | Per-workload uptime percentage (0.0 -- 100.0) |
| `restart_counts` | Health history | Per-workload cumulative restart count |
| `drift_detected` | Drift engine | Whether any workload has configuration drift |
| `policy_violations` | Policy engine | Whether any policy violations exist |
| `secrets_expiring_days` | Secrets store | Per-secret days until expiry |

### Alert Conditions

| Condition | Triggers When |
|---|---|
| `SlaUptimeBelow(threshold)` | Any workload's uptime drops below the threshold (e.g., 99.9%) |
| `ExcessiveRestarts(max)` | Any workload exceeds the maximum restart count |
| `DriftDetected` | Configuration drift is found on any workload |
| `PolicyViolation` | Any active policy violation exists |
| `SecretExpiring(days)` | Any secret expires within the specified number of days |
| `ErrorRateAbove(rate)` | Fleet or workload health failure rate exceeds `rate` (0.0–1.0) |
| `CostExceeds(amount)` | Monthly priced cost for fleet or a workload exceeds `amount` (USD) |

### Cooldown

Each alert rule has a `cooldown_seconds` field that prevents repeated firing. After an alert fires, it will not fire again until the cooldown period has elapsed, even if the condition remains true.

### Integration with Orchestrate Watch

Alert evaluation happens automatically during each `orchestrate watch` cycle:

```bash
# Alerts are evaluated every 30 seconds (default interval)
aether orchestrate watch

# Alerts are evaluated every 10 seconds
aether orchestrate watch --interval 10
```

When alerts fire, they appear in the event stream:

```bash
aether events --severity warning
```

Alerts also trigger any configured webhook notification channels.

---

## 🔗 Cross-References

| Document | Relevance |
|---|---|
| [Kubernetes Guide](../../KUBERNETES.md) | Kubernetes health probes and volume mounts |
| [Security Guide](./security.md) | Secrets and policy enforcement |
| [Compose Guide](./compose.md) | Multi-workload deployments |
| [Plugin System](./plugins.md) | Custom runtime health checks |
| [API Reference](../reference/api/API-Reference.md) | Full REST API documentation |
| [Quick Reference](../quick-reference/QUICK_REFERENCE.md) | Command cheat sheet |
