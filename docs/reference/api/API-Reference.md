# REST API Reference

Complete API reference for the Orchestr8 REST server.

---

## Server Setup

```bash
orchestr8 serve                          # Default: 127.0.0.1:8080
orchestr8 serve --host 0.0.0.0 --port 3000
```

## Response Format

All responses use a standard envelope:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

---

## Workload Endpoints

### List Workloads
```
GET /api/workloads
```
```bash
curl http://localhost:8080/api/workloads
```

### Create Workload
```
POST /api/workloads
Content-Type: application/json
Body: { "spec": "<yaml string>", "runtime": "podman" }
```

### Get Workload Details
```
GET /api/workloads/:name
```

### Get Workload Logs
```
GET /api/workloads/:name/logs
```

### Stop Workload
```
POST /api/workloads/:name/stop
```

### Delete Workload
```
DELETE /api/workloads/:name
```

### Migrate Workload
```
POST /api/workloads/:name/migrate
Content-Type: application/json
Body: { "target_runtime": "kubernetes", "strategy": "blue-green" }
```

---

## Cost Estimation

### Estimate Cost
```
POST /api/cost
Content-Type: application/json
Body: { "spec": "<yaml string>", "provider": "aws" }
```

---

## Backups

### List Backups
```
GET /api/backups
```

### Create Backup
```
POST /api/backups
Content-Type: application/json
Body: { "name": "my-backup", "description": "Pre-deploy backup" }
```

---

## Plugins

### List Plugins
```
GET /api/plugins
```

### Discover Plugins
```
POST /api/plugins/discover
```

---

## Health

### Health Summary
```
GET /api/health/:workload
```

Returns: `{ total_checks, ready_checks, uptime_percent, last_state, last_restart_count }`

---

## Compose

### Validate Compose
```
POST /api/compose/validate
Content-Type: text/plain
Body: <YAML compose spec>
```

Returns: `{ valid: true, workload_count: 3, deploy_order: ["db", "api", "web"] }`

---

## Other Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/metrics` | Prometheus metrics |
| GET | `/api/scheduler/utilization` | Runtime utilization |
| GET | `/api/scheduler/optimize` | Optimization suggestions |
| GET | `/api/orchestrator/status` | Health orchestrator status |
| GET | `/api/orchestrator/summary` | Health summary |
| GET | `/api/affinity/:class` | Affinity recommendations |
| GET | `/api/secrets` | Secret summaries |
| GET | `/api/events` | Event log |
| GET | `/api/events/summary` | Event summary |
| GET | `/api/environments` | Environment list |
| GET | `/api/drift/:name` | Drift report |
| GET | `/` | Web dashboard (HTML) |
