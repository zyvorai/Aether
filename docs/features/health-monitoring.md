# Health Monitoring 💓

Track workload uptime, restart history, and health timelines.

---

## Overview

Every `orchestr8 status` call automatically records a health check observation. These observations accumulate in a bounded history (max 1000 records) stored at `~/.orchestr8/health.json`.

---

## Health Record Fields

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | RFC 3339 | When the check occurred |
| `workload` | String | Workload name |
| `runtime` | RuntimeKind | Current runtime |
| `state` | InstanceState | running, stopped, failed, unknown |
| `ready` | bool | Whether the workload is healthy |
| `restart_count` | u32 | Cumulative restart count |
| `latency_ms` | Option<f64> | Health probe latency |

---

## Commands

### Timeline View

```bash
orchestr8 health my-app --last 20
```

```
┌─────────────────────┬─────────┬───────┬──────────┬─────────┐
│ Timestamp           │ State   │ Ready │ Restarts │ Latency │
├─────────────────────┼─────────┼───────┼──────────┼─────────┤
│ 2026-04-11T10:00:00 │ running │ ●     │ 0        │ 12ms    │
│ 2026-04-11T10:05:00 │ running │ ●     │ 0        │ 15ms    │
│ 2026-04-11T10:10:00 │ failed  │ ○     │ 1        │ -       │
│ 2026-04-11T10:15:00 │ running │ ●     │ 1        │ 18ms    │
└─────────────────────┴─────────┴───────┴──────────┴─────────┘
  Uptime: 75.00%  |  3 / 4 checks ready  |  1 restarts
```

### Summary View

```bash
orchestr8 health my-app --summary
```

### JSON Output

```bash
orchestr8 health my-app --summary --output json
```

---

## Uptime Calculation

Uptime = (ready checks / total checks) * 100%

- Only checks for the specified workload are counted
- Records are pruned when the history exceeds `max_records` (1000)

---

## Integration Points

- **Status command:** Auto-records a health observation on each `orchestr8 status` call
- **TUI dashboard:** Shows resource details and health bar in the detail panel
- **Background loop:** `orchestrate watch` runs periodic health checks
- **SLA monitoring:** Background loop checks uptime against SLA thresholds

---

## REST API

```bash
curl http://localhost:8080/api/health/my-app
```

Returns `HealthSummary` with total_checks, ready_checks, uptime_percent, last_state, last_restart_count.
