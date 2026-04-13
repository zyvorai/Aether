# 🚀 Deployment Patterns

> Common deployment patterns and best practices for Aether workloads.

---

## 📑 Table of Contents

- [Single Workload Deployment](#-single-workload-deployment)
- [Directory Batch Deploy](#-directory-batch-deploy)
- [Compose Multi-Workload Stacks](#-compose-multi-workload-stacks)
- [Template-Based Deployment](#-template-based-deployment)
- [Environment Promotion](#-environment-promotion)
- [CI/CD Pipeline Deployment](#-cicd-pipeline-deployment)
- [Watch Mode (Hot Reload)](#-watch-mode-hot-reload)

---

## 📦 Single Workload Deployment

The simplest pattern: one workload spec, one deployment.

```bash
# Validate → Build → Deploy
aether validate
aether build
aether run

# Or with runtime override
aether run --runtime kubernetes

# Dry-run to preview
aether --dry-run run --runtime kube
```

### Lifecycle

```bash
aether status my-app     # Check status
aether logs my-app       # View logs
aether logs my-app -f    # Follow logs
aether exec my-app       # Shell into container
aether stop my-app       # Stop
aether delete my-app     # Remove completely
```

---

## 📁 Directory Batch Deploy

Deploy all workload specs from a directory at once.

```bash
# Deploy all YAML files in specs/
aether deploy ./specs/

# With runtime override
aether deploy ./specs/ --runtime kubernetes

# Stop on first failure
aether deploy ./specs/ --fail-fast

# Preview without deploying
aether deploy ./specs/ --dry-run
```

The deploy command:
1. Discovers all `*.yaml` files in the directory
2. Validates each spec
3. Deploys in parallel (or sequentially with `--fail-fast`)
4. Reports results summary

---

## 🎼 Compose Multi-Workload Stacks

For applications with multiple services and dependencies.

### Compose File

```yaml
# aether-compose.yaml
version: "1"
workloads:
  frontend:
    spec: ./frontend/workload.yaml
    runtime: podman
    depends_on:
      - backend
    environment:
      API_URL: http://backend:8080
  backend:
    spec: ./backend/workload.yaml
    runtime: kubernetes
    depends_on:
      - database
  database:
    spec: ./database/workload.yaml
    runtime: kubernetes
```

### Commands

```bash
aether compose validate              # Validate compose file
aether compose up                    # Deploy all (respects dependency order)
aether compose up --runtime kube     # Override runtime for all
aether compose up --dry-run          # Preview
aether compose down                  # Stop all
```

---

## 📝 Template-Based Deployment

Generate workload specs from 8 built-in templates.

```bash
# List available templates
aether template --list

# Generate from template
aether template web-app --workload-name my-site --owner team --project demo

# Save to file
aether template rest-api --workload-name api-svc -o api.yaml

# Then deploy
aether -s api.yaml run
```

### Available Templates

| Template | Ports | Use Case |
|----------|-------|----------|
| `web-app` | 80, 443 | Web frontends |
| `rest-api` | 8080 | API backends |
| `database` | 5432 | PostgreSQL, MySQL |
| `cache` | 6379 | Redis, Memcached |
| `worker` | — | Background jobs |
| `cron-job` | — | Scheduled tasks |
| `ml-training` | — | GPU workloads |
| `microservice` | 8080 | Generic microservice |

---

## 🔄 Environment Promotion

Promote workloads through environments (dev → staging → production).

```bash
# Create environments
aether env create dev --tier development
aether env create staging --tier staging
aether env create prod --tier production

# Promote
aether env promote my-app dev staging
aether env promote my-app staging prod

# Check parity
aether env parity staging prod
```

---

## 🔁 CI/CD Pipeline Deployment

### GitHub Actions

```yaml
- name: Deploy
  run: |
    aether validate -s workload.yaml
    aether policy-check -s workload.yaml --policy production
    aether --yes run -s workload.yaml --runtime kubernetes
```

### Key Flags for CI

| Flag | Purpose |
|------|---------|
| `--yes` | Auto-confirm prompts |
| `--json` | Machine-readable output |
| `--quiet` | Suppress non-error output |
| `--dry-run` | Preview without executing |
| `--output json` | JSON output format |

---

## 👁 Watch Mode (Hot Reload)

Auto-redeploy when spec file changes (development).

```bash
aether watch
aether watch --runtime podman
```

Watch mode:
1. Polls the spec file every 500ms
2. Debounces changes (waits 1s after last change)
3. Reads file atomically to prevent partial reads
4. Validates, then redeploys

---

## 🔗 Related Documentation

| Document | Description |
|----------|-------------|
| [Quick Start](../getting-started/02-Quick-Start.md) | 5-minute getting started |
| [Compose Guide](../features/compose.md) | Multi-workload deployment details |
| [CI/CD Guide](../CICD.md) | Pipeline integration |
| [Templates](../TEMPLATES.md) | Template catalog |
