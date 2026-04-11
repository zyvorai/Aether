# Tutorial: Advanced Features 🔬

**Duration:** 60 minutes | **Level:** Advanced

Master policies, encryption, drift reconciliation, plugins, and automation.

---

## 1. Policy Engine 📋

### Check Policies Manually

```bash
orchestr8 policy-check --spec app.yaml --policy production
orchestr8 policy-check --spec app.yaml --policy development
```

### Automatic Policy Gate

Policies are enforced automatically on every deploy. Configure in `~/.orchestr8/config.yaml`:

```yaml
policy:
  enforceOnDeploy: true
  policySet: production    # "production", "development", or file path
```

Skip temporarily with:
```bash
orchestr8 run --spec app.yaml --skip-policy
```

### Built-in Policy Rules

| Rule | Production Default | Description |
|------|-------------------|-------------|
| MaxCpu | 64 cores | CPU limit |
| MaxMemoryGi | 256 GiB | Memory limit |
| MaxStorageGi | 1000 GiB | Storage limit |
| MaxGpu | 8 | GPU count limit |
| RequireTls | Enabled | TLS on ingress |
| RequireOwner | Enabled | Owner metadata required |
| RequireResourceLimits | Enabled | CPU/memory must be set |

---

## 2. AES-256-GCM Secrets 🔐

### Set a Production Key

```bash
export ORCHESTR8_SECRET_KEY="my-secure-key-at-least-16-chars"
```

### Manage Secrets

```bash
# Create a secret namespace
orchestr8 secrets create db-creds --namespace production

# Set values (encrypted with AES-256-GCM)
orchestr8 secrets set db-creds password "s3cret!"
orchestr8 secrets set db-creds connection-string "postgres://..."

# Retrieve (decrypted)
orchestr8 secrets get db-creds password

# Rotate
orchestr8 secrets set db-creds password "new-s3cret!"

# Audit access
orchestr8 secrets audit db-creds

# List all
orchestr8 secrets list
```

> **Note:** Without `ORCHESTR8_SECRET_KEY`, secrets use XOR obfuscation (dev-only). Set the env var for AES-256-GCM encryption in production.

---

## 3. Drift Detection & Reconciliation 🔍

```bash
# Detect drift
orchestr8 drift my-app

# Auto-fix drift (with confirmation)
orchestr8 drift my-app --reconcile
```

Drift categories: Runtime, Image, Resources, Network, Configuration, Scaling, Health.

---

## 4. Plugin System 🔌

### Discover Plugins

```bash
orchestr8 plugin discover    # Scan ~/.orchestr8/plugins/
orchestr8 plugin list        # Show registered plugins
```

### Create a Plugin Manifest

Save to `~/.orchestr8/plugins/wasm-runtime.json`:

```json
{
  "name": "wasm-runtime",
  "version": "0.1.0",
  "runtime_kind": "wasm",
  "command": "/usr/local/bin/wasm-adapter",
  "capabilities": ["build", "run", "stop", "status"]
}
```

```bash
orchestr8 plugin discover
orchestr8 plugin list
```

---

## 5. Webhook Notifications 🔔

```bash
# Add a webhook channel
orchestr8 webhook add alerts https://hooks.slack.com/... --severity warning

# Test it
orchestr8 webhook test alerts

# View retry queue
orchestr8 webhook queue

# Force retry all queued
orchestr8 webhook flush
```

Webhooks use persistent retry with exponential backoff (30s → 1m → 2m → 4m → 8m).

---

## 6. Background Reconciliation Loop 🔄

```bash
orchestr8 orchestrate watch --interval 10
```

Runs automated checks on configurable intervals:

| Check | Default Interval | Action |
|-------|-----------------|--------|
| Health | 30s | Circuit breaker, auto-restart |
| Drift | 300s | Detect divergence, emit events |
| SLA | 60s | Uptime monitoring, alerts |
| Webhooks | Every tick | Process retry queue |

Configure in `~/.orchestr8/config.yaml`:

```yaml
reconciliation:
  healthIntervalSecs: 30
  driftIntervalSecs: 300
  slaIntervalSecs: 60
  autoReconcile: false
```

---

## 7. Cost Estimation 💰

```bash
orchestr8 cost --spec app.yaml --provider aws
orchestr8 cost --spec app.yaml --provider all
```

Compares: AWS, Azure, GCP, DigitalOcean, Linode.

---

## 8. AI Recommendations 🤖

```bash
orchestr8 recommend --spec app.yaml         # Runtime recommendation
orchestr8 scaling-advice                     # Scaling suggestions
orchestr8 migration-advice my-app kubernetes # Migration strategy advice
orchestr8 profile --spec app.yaml            # Workload profiling
```

---

## Next Steps

- [REST API Reference](../reference/api/API-Reference.md) — Build integrations
- [Security Guide](../features/security.md) — Deep dive into encryption and policies
- [CLI Reference](../guides/cli/CLI-Reference.md) — Complete command reference
