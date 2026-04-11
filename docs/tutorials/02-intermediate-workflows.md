# Tutorial: Intermediate Workflows 🔧

**Duration:** 45 minutes | **Level:** Intermediate

Learn compose files, migrations, output formats, and monitoring.

---

## 1. Multi-Workload Compose Files 📦

Create `orchestr8-compose.yaml`:

```yaml
version: "1"
workloads:
  database:
    spec: db.yaml
    runtime: container
    env:
      POSTGRES_DB: myapp
      POSTGRES_USER: admin
  api:
    spec: api.yaml
    depends_on: [database]
    env:
      DATABASE_URL: postgres://admin@database:5432/myapp
  web:
    spec: web.yaml
    depends_on: [api]
```

### Deploy in Dependency Order

```bash
# Validate first
orchestr8 compose validate

# Deploy all (db → api → web)
orchestr8 compose up

# Preview without executing
orchestr8 compose up --dry-run

# Tear down (web → api → db)
orchestr8 compose down
```

---

## 2. Runtime Migration 🔄

### Compare Runtimes First

```bash
orchestr8 compare
```

Shows suitability, limitations, and cost for each runtime.

### Migration Strategies

| Strategy | Downtime | Resources | Best For |
|----------|----------|-----------|----------|
| `immediate` | Brief (~seconds) | 1x | Dev/test |
| `blue-green` | Zero | 2x temporarily | Production |
| `rolling` | Zero | 1.5x gradually | Critical services |

### Execute a Migration

```bash
# Blue-green (recommended for production)
orchestr8 migrate my-app kubernetes --strategy blue-green

# Rolling (gradual traffic shift: 25% → 50% → 75% → 100%)
orchestr8 migrate my-app kubevirt --strategy rolling

# Preview
orchestr8 migrate my-app kubernetes --dry-run
```

---

## 3. Output Formats 📊

```bash
# Default table
orchestr8 list

# Wide table (extra columns)
orchestr8 list --output wide

# JSON (for scripting)
orchestr8 status my-app --output json

# YAML
orchestr8 status my-app --output yaml
```

---

## 4. Dry-Run Mode 🔍

Preview any mutating command without executing:

```bash
orchestr8 run --spec app.yaml --dry-run
orchestr8 migrate my-app kubernetes --dry-run
orchestr8 stop my-app --dry-run
```

---

## 5. Health Monitoring 💓

```bash
# View health timeline
orchestr8 health my-app --last 20

# Summary only
orchestr8 health my-app --summary

# JSON output for dashboards
orchestr8 health my-app --summary --output json
```

---

## 6. Watch Mode 👁️

Auto-redeploy when your spec file changes:

```bash
orchestr8 watch --spec app.yaml
# Press Ctrl+C to stop
```

Includes 1-second debounce to handle editor save patterns.

---

## 7. Exec and Port-Forward 🐚

```bash
# Shell into a running workload
orchestr8 exec my-app
orchestr8 exec my-app -i /bin/bash

# Forward local ports
orchestr8 port-forward my-app 8080:80
```

---

## Next Steps

- [Advanced Features](03-advanced-features.md) — Policies, secrets, drift, plugins
- [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md) — Production migration guide
