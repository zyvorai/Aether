# 🚀 Quick Start Guide

> Deploy your first workload in under 5 minutes. From YAML spec to running instance
> with status, logs, migration, and a TUI dashboard.

---

## 📖 Table of Contents

- [Prerequisites Checklist](#-prerequisites-checklist)
- [Step 1: Install aether](#-step-1-install-aether)
- [Step 2: Create Your First Workload Spec](#-step-2-create-your-first-workload-spec)
- [Step 3: Validate the Spec](#-step-3-validate-the-spec)
- [Step 4: Build the Image](#-step-4-build-the-image)
- [Step 5: Deploy the Workload](#-step-5-deploy-the-workload)
- [Step 6: Check Status and View Logs](#-step-6-check-status-and-view-logs)
- [Step 7: Migrate to Another Runtime](#-step-7-migrate-to-another-runtime)
- [Step 8: Launch the TUI Dashboard](#-step-8-launch-the-tui-dashboard)
- [Step 9: Explore More Features](#-step-9-explore-more-features)
- [Step 10: Clean Up](#-step-10-clean-up)
- [Next Steps](#-next-steps)

---

## ✅ Prerequisites Checklist

Before you begin, make sure you have:

- [ ] **aether** installed ([Installation Guide](01-Installation.md))
- [ ] **Podman** available (recommended for local dev): `podman --version`
- [ ] A text editor for YAML files
- [ ] A simple `Dockerfile` in your project directory (or use the example below)

> **Don't have Podman?** Install it with `sudo dnf install podman` (Fedora) or
> `sudo apt install podman` (Ubuntu/Debian).

---

## 📥 Step 1: Install aether

If you haven't installed yet:

```bash
# Build from source (fastest path)
git clone https://github.com/ssahani/aether.git
cd aether
cargo build --release
sudo install -m 0755 target/release/aether /usr/local/bin/aether

# Run the setup wizard
aether init
```

Verify:

```bash
aether --version
# aether 0.3.0
```

---

## 📝 Step 2: Create Your First Workload Spec

Create a file called `workload.yaml` in your project directory:

```yaml
apiVersion: aether/v1
kind: Workload

metadata:
  name: hello-web
  owner: my-team
  project: quickstart
  labels:
    environment: development
    tier: frontend

build:
  context: .
  dockerfile: Dockerfile
  registry: localhost

requirements:
  cpu: "1"
  memory: 512Mi
  storage: 1Gi

runtime:
  preferred: auto
  allow:
    - container
    - kube

network:
  service: true
  serviceType: ClusterIP
  ports:
    - containerPort: 8080
      servicePort: 8080
      protocol: TCP

health:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10

scaling:
  minReplicas: 1
  maxReplicas: 4
  targetCPUPercent: 70
```

If you don't have a `Dockerfile`, create a minimal one:

```dockerfile
FROM docker.io/library/nginx:alpine
COPY . /usr/share/nginx/html
EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]
```

> **Tip:** You can also generate a spec from a built-in template:
>
> ```bash
> aether template web-app --workload-name hello-web --output workload.yaml
> ```

---

## ✔️ Step 3: Validate the Spec

Check that your YAML is structurally correct:

```bash
aether validate
```

**Expected output:**

```
  Validation passed  hello-web
```

Validation checks:
- Required fields are present (`apiVersion`, `kind`, `metadata`, `build`, `requirements`, `runtime`)
- Resource values are parseable (`cpu`, `memory`, `storage`)
- Runtime preferences reference valid targets
- Port mappings are consistent

### Validate with a Specific File

```bash
aether validate --spec path/to/my-workload.yaml
```

---

## 🔨 Step 4: Build the Image

Build the container image from your Dockerfile:

```bash
aether build
```

**Expected output:**

```
  Building image  hello-web:latest
  Build complete  localhost/hello-web:latest (podman)
```

Aether uses the runtime's native build tooling:
- **Podman** -- `podman build`
- **Kubernetes** -- builds locally and pushes to the configured registry

---

## 🚀 Step 5: Deploy the Workload

Deploy your workload to a runtime:

```bash
# Auto-select the best runtime (AI-scored)
aether run

# Or specify a runtime explicitly
aether run --runtime podman

# Preview without deploying (dry run)
aether run --dry-run
```

**Expected output:**

```
  Deploying  hello-web to podman
  Runtime    podman (auto-selected, score: 92/100)
  Instance   hello-web-a1b2c3
  Status     running
```

### Runtime Selection

When you use `--runtime auto` (the default for `preferred: auto`), aether's
AI scoring engine evaluates all allowed runtimes and picks the best match based
on your resource requirements, runtime availability, and workload characteristics.

You can see the full scoring breakdown with:

```bash
aether recommend
```

---

## 📊 Step 6: Check Status and View Logs

### Status

```bash
aether status hello-web
```

**Expected output:**

```
  Workload  hello-web
  Runtime   podman
  State     running
  Ready     true
  Restarts  0
```

### Logs

```bash
# View recent logs
aether logs hello-web

# Follow logs in real time
aether logs hello-web --follow
```

### List All Workloads

```bash
# Table format (default)
aether list

# JSON for scripting
aether list --output json

# YAML format
aether list --output yaml

# Wide table with extra columns
aether list --output wide
```

**Example table output:**

```
 Name        Runtime    Image                    Status    Created
 hello-web   podman     localhost/hello-web:latest running  2026-04-11T10:30:00Z
```

---

## 🔄 Step 7: Migrate to Another Runtime

Move your workload from one runtime to another with zero downtime:

```bash
# Blue-green migration (recommended for production)
aether migrate hello-web kubernetes --strategy blue-green

# Rolling migration (gradual traffic shift)
aether migrate hello-web kube --strategy rolling

# Immediate migration (brief downtime, fastest)
aether migrate hello-web kube --strategy immediate
```

**Expected output (blue-green):**

```
  Migrating    hello-web: podman -> kubernetes (blue-green)
  Phase 1/3    Deploying to target runtime (green deployment)
  Phase 2/3    Validating green deployment... healthy
  Phase 3/3    Switching traffic, stopping blue deployment
  Complete     Migration successful (12.3s)
```

### Migration Strategies Compared

| Strategy | Downtime | How It Works |
|----------|----------|-------------- |
| **Blue-Green** | Zero | Deploys target alongside source, validates, switches traffic, then stops source |
| **Rolling** | Zero | Deploys target, validates with retries, shifts traffic 25% -> 50% -> 75% -> 100% |
| **Immediate** | Brief | Stops source first, then starts target. Rollback on failure if enabled |

### Get AI Migration Advice

Before migrating, ask for recommendations:

```bash
aether migration-advice hello-web kubernetes
```

This shows recommended strategy, estimated downtime, risk level, and timing advice.

### Verify After Migration

```bash
aether status hello-web
# Runtime should now show "kubernetes"

aether diff hello-web
# Compare spec vs stored vs live state
```

---

## 🖥️ Step 8: Launch the TUI Dashboard

Open the interactive terminal UI for a real-time view of all workloads:

```bash
aether tui
```

### TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate workload list |
| `/` | Search and filter workloads |
| `Enter` | View logs for selected workload |
| `r` | Refresh data |
| `s` | Show status details |
| `q` / `Esc` | Quit dashboard |

> **Alternative:** Start the web dashboard instead:
>
> ```bash
> aether serve --port 8080
> # Open http://localhost:5090 in your browser
> ```

---

## 🔍 Step 9: Explore More Features

Now that you have a running workload, try these features:

### Cost Estimation

```bash
# Compare costs across cloud providers
aether cost --provider all
```

### Health Check History

```bash
# View health timeline
aether health hello-web

# Summary only
aether health hello-web --summary
```

### Drift Detection

```bash
# Check if live state matches spec
aether drift hello-web

# Auto-reconcile any drift
aether drift hello-web --reconcile
```

### Policy Check

```bash
# Check against production policies
aether policy-check --policy production
```

### Deploy Multiple Workloads with Compose

Create `aether-compose.yaml`:

```yaml
version: "1"
workloads:
  database:
    spec: ./specs/database.yaml
    runtime: container
    env:
      POSTGRES_DB: myapp
      POSTGRES_USER: admin

  api:
    spec: ./specs/api.yaml
    runtime: kube
    depends_on:
      - database
    env:
      DATABASE_URL: postgres://admin@database:5432/myapp

  web:
    spec: ./specs/web.yaml
    depends_on:
      - api
```

```bash
# Validate the compose file
aether compose validate

# Deploy all workloads in dependency order
aether compose up

# Tear down everything
aether compose down
```

### Backup State

```bash
# Create a named backup
aether backup --name before-upgrade --description "Pre-upgrade snapshot"

# List backups
aether list-backups

# Restore if needed
aether restore ~/.aether/backups/before-upgrade.json
```

### Secrets Management

```bash
# Create a secret namespace
aether secrets create app-secrets --namespace production

# Store encrypted values (AES-256)
aether secrets set app-secrets DB_PASSWORD "s3cure-p@ss"
aether secrets set app-secrets API_KEY "sk-12345"

# Retrieve a value
aether secrets get app-secrets DB_PASSWORD

# Audit rotation status
aether secrets audit
```

---

## 🧹 Step 10: Clean Up

When you're done experimenting:

```bash
# Stop the workload
aether stop hello-web

# Delete the workload and clean up
aether delete hello-web

# Verify
aether list
# (empty)
```

---

## 🔗 Next Steps

| What | Where |
|------|-------|
| Detailed beginner tutorial | [Beginner Tutorial](../tutorials/01-beginner-deployment.md) |
| Multi-workload compose | [Compose Guide](../features/compose.md) |
| All CLI commands | [CLI Reference](../guides/cli/CLI-Reference.md) or `aether help-all` |
| Migration checklist | [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md) |
| REST API and web dashboard | [Web UI Guide](../WEBUI.md) |
| Security and secrets | [Security Guide](../features/security.md) |
| CI/CD integration | [CI/CD Guide](../CICD.md) |
| Full documentation index | [Documentation Index](../index.md) |
| Documentation hub | [README](../README.md) |

---

## 🃏 Quick Command Reference

```bash
# Lifecycle
aether init                          # Setup wizard
aether validate                      # Validate spec
aether build                         # Build image
aether run [--runtime <rt>]          # Deploy
aether status <name>                 # Check status
aether logs <name> [--follow]        # View logs
aether stop <name>                   # Stop
aether delete <name>                 # Delete
aether list                          # List all

# Migration
aether migrate <name> <target> --strategy blue-green
aether migration-advice <name> <target>
aether rollback <name>

# Observability
aether tui                           # Terminal dashboard
aether serve                         # Web dashboard + API
aether health <name>                 # Health history
aether metrics                       # Prometheus export

# Operations
aether compose up                    # Deploy stack
aether backup                        # Backup state
aether secrets list                  # List secrets
aether drift <name>                  # Detect drift
aether policy-check                  # Check policies
```

---

*aether v0.3.0 -- Universal Runtime Control Plane*
