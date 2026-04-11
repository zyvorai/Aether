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

orchestr8's health monitoring system records periodic health check observations for each workload, building a historical timeline that enables:

- 📈 **Uptime percentage** calculations (ratio of ready checks to total checks)
- 🔄 **Restart tracking** across time
- 🕐 **Health timeline** showing state transitions
- 📋 **Summary statistics** per workload
- 🗂️ **Bounded storage** with automatic pruning of old records

Health data is stored in `~/.orchestr8/health.json` and is updated whenever you run status checks, background watches, or orchestration health checks.

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
orchestr8 health api-service                  # Default: last 20 records
orchestr8 health api-service --last 10        # Last 10 records
orchestr8 health api-service --last 50        # Last 50 records
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
orchestr8 health api-service --summary
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

## 🔄 Integration with Status Command

The `orchestr8 status <name>` command automatically records a health check when it queries a workload's status. This means every status check contributes to the health history.

```bash
# This queries status AND records a health check
orchestr8 status my-app

# View the accumulated history
orchestr8 health my-app
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

Use `orchestr8 orchestrate watch` to run continuous health monitoring at a configurable interval.

```bash
# Check every 30 seconds (default)
orchestr8 orchestrate watch

# Check every 10 seconds
orchestr8 orchestrate watch --interval 10

# Check every 5 minutes
orchestr8 orchestrate watch --interval 300
```

Each watch cycle:

1. Queries status of **all registered workloads** across all runtimes
2. Records a `HealthRecord` for each workload
3. Saves the updated health history to disk
4. Triggers alerts if circuit breakers trip

### One-Shot Health Check

Run a single round of health checks against all live runtimes without entering a continuous loop:

```bash
orchestr8 orchestrate health-check
```

### Other Orchestration Commands

```bash
orchestr8 orchestrate register my-app --runtime kubernetes  # Register for monitoring
orchestr8 orchestrate status                                 # Show all health statuses
orchestr8 orchestrate summary                                # Health summary
orchestr8 orchestrate reset-circuit my-app                   # Reset circuit breaker
```

---

## 🖥️ Health in the TUI Dashboard

Launch the TUI with `orchestr8 tui` to see a real-time dashboard with health information.

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
curl http://localhost:8080/api/health/api-service
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
~/.orchestr8/health.json
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
| Default path | `~/.orchestr8/health.json` (auto-detected from `$HOME`) |

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

## 🔗 Cross-References

| Document | Relevance |
|---|---|
| [Security Guide](./security.md) | Secrets and policy enforcement |
| [Compose Guide](./compose.md) | Multi-workload deployments |
| [Plugin System](./plugins.md) | Custom runtime health checks |
| [API Reference](../reference/api/API-Reference.md) | Full REST API documentation |
| [Quick Reference](../quick-reference/QUICK_REFERENCE.md) | Command cheat sheet |
