# ⚡ orchestr8 Quick Reference Card

> One-page cheat sheet for the Universal Runtime Control Plane.

---

## 📑 Table of Contents

- [Essential Commands](#-essential-commands)
- [Output Formats](#-output-formats)
- [Runtime Icons](#-runtime-icons)
- [Migration Strategies](#-migration-strategies)
- [TUI Keyboard Shortcuts](#-tui-keyboard-shortcuts)
- [Configuration](#-configuration)
- [Environment Variables](#-environment-variables)
- [Common Flags](#-common-flags)
- [Compose Commands](#-compose-commands)
- [Plugin Commands](#-plugin-commands)
- [Health Commands](#-health-commands)
- [Secrets Commands](#-secrets-commands)
- [AI and Intelligence](#-ai-and-intelligence)
- [More Documentation](#-more-documentation)

---

## 🎯 Essential Commands

### Workload Lifecycle

```bash
orchestr8 validate                      # Validate workload spec
orchestr8 build                         # Build container image
orchestr8 run                           # Deploy (auto-select runtime)
orchestr8 run --runtime kube            # Deploy to specific runtime
orchestr8 status my-app                 # Check status
orchestr8 logs my-app                   # View logs
orchestr8 logs my-app --follow          # Stream logs in real-time
orchestr8 stop my-app                   # Stop workload
orchestr8 delete my-app                 # Delete workload
orchestr8 list                          # List all workloads
```

### Migration

```bash
orchestr8 migrate my-app kube                     # Migrate (default: blue-green)
orchestr8 migrate my-app kube -s immediate        # Immediate migration
orchestr8 migrate my-app kube -s rolling          # Rolling migration
orchestr8 migrate my-app metal --no-rollback      # Disable automatic rollback
orchestr8 migrate my-app kube --no-validation     # Skip validation delay
```

### Custom Spec File

```bash
orchestr8 -s ./my-app.yaml validate              # Use custom spec file
orchestr8 -s ./my-app.yaml run --runtime podman   # Deploy custom spec
```

### Dashboard and API

```bash
orchestr8 tui                           # Launch interactive TUI dashboard
orchestr8 serve                         # Start REST API server (localhost:8080)
orchestr8 serve --host 0.0.0.0 -p 3000 # Bind to all interfaces, custom port
```

### Backup and Restore

```bash
orchestr8 backup                        # Create backup (auto-named)
orchestr8 backup -n pre-deploy          # Named backup
orchestr8 backup -d "Before migration"  # Backup with description
orchestr8 list-backups                  # List all backups
orchestr8 restore ./backup.json         # Restore from backup
orchestr8 restore ./backup.json --merge # Merge with existing state
```

### Batch Operations

```bash
orchestr8 deploy ./specs/               # Deploy all YAML in directory
orchestr8 deploy ./specs/ --fail-fast   # Stop on first failure
orchestr8 deploy ./specs/ --dry-run     # Preview deployment plan
orchestr8 deploy ./specs/ -r kube       # Override runtime for all
```

### Advanced Operations

```bash
orchestr8 exec my-app                   # Shell into workload (/bin/sh)
orchestr8 exec my-app "ls -la"          # Run command in workload
orchestr8 port-forward my-app 8080:80   # Forward local:remote ports
orchestr8 watch                         # Auto-redeploy on spec changes
orchestr8 diff my-app                   # Compare spec vs live state
orchestr8 rollback my-app               # Rollback to latest snapshot
orchestr8 compare                       # Compare runtimes for spec
orchestr8 init                          # First-time setup wizard
```

---

## 📄 Output Formats

| Flag | Format | Description |
|---|---|---|
| `-o table` | Table | Default. Colored, human-readable tables with Unicode borders |
| `-o json` | JSON | Machine-readable JSON output |
| `-o yaml` | YAML | YAML output |
| `-o wide` | Wide | Extra columns with additional detail |
| `--json` | JSON | Shorthand for `-o json` |
| `-q` / `--quiet` | Quiet | Suppress all output except errors |

### Examples

```bash
orchestr8 list -o json                  # JSON output
orchestr8 list -o yaml                  # YAML output
orchestr8 list -o wide                  # Wide table with extra columns
orchestr8 list --quiet                  # Errors only
orchestr8 list --json                   # Shorthand JSON
```

---

## 🎨 Runtime Icons

| Icon | Runtime | CLI Aliases | Description |
|---|---|---|---|
| 🐳 | Podman | `podman`, `container` | Container runtime |
| ☸️ | Kubernetes | `kubernetes`, `kube`, `k8s` | Container orchestration |
| 🖥️ | KubeVirt | `kubevirt`, `vm` | Virtual machines on Kubernetes |
| 🖧 | Metal3 | `metal3`, `metal`, `bare-metal` | Bare-metal provisioning |

---

## 🔄 Migration Strategies

### Comparison Table

| Feature | `immediate` | `blue-green` | `rolling` |
|---|---|---|---|
| **Downtime** | Yes (brief) | ~Zero | ~Zero |
| **Safety** | ⚠️ Low | ✅ High | ✅ Highest |
| **Default** | No | **Yes** | No |
| **Rollback** | ✅ Supported | ✅ Supported | ✅ Supported |
| **Health retries** | 1 | 1 | 3 (exponential backoff) |
| **Traffic shift** | Instant | Instant | Gradual (25→50→75→100%) |
| **Resources during migration** | 1x | 2x (both running) | 2x (both running) |
| **Best for** | Dev/test | Production | Critical production |

### How Each Strategy Works

| Strategy | Steps |
|---|---|
| `immediate` | Stop source → Build on target → Deploy on target → Validate → Cleanup |
| `blue-green` | Build on target (green) → Deploy green → Validate green → Switch traffic → Stop source (blue) → Cleanup |
| `rolling` | Deploy on target → Validate with retries → Shift 25% → Shift 50% → Shift 75% → Shift 100% → Cleanup source |

---

## ⌨️ TUI Keyboard Shortcuts

### Dashboard View

| Key | Action |
|---|---|
| `q` / `Ctrl+C` | Quit the TUI |
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `Enter` | View logs for selected workload |
| `r` | Refresh workload statuses |
| `/` | Toggle search/filter mode |

### Logs View

| Key | Action |
|---|---|
| `Esc` | Return to dashboard |
| `q` | Quit |

### Search Mode

| Key | Action |
|---|---|
| (type text) | Filter workloads by name |
| `Esc` | Clear search and exit search mode |

---

## ⚙️ Configuration

### Config File Location

```
~/.orchestr8/config.yaml
```

### Config Commands

```bash
orchestr8 config --show                 # Display current configuration
orchestr8 config --init                 # Create default config file
```

### State and Data Files

| File | Purpose |
|---|---|
| `~/.orchestr8/config.yaml` | Configuration file |
| `~/.orchestr8/state.json` | Workload state database (file-locked, atomic writes) |
| `~/.orchestr8/health.json` | Health check history (bounded ring buffer, max 1000) |
| `~/.orchestr8/secrets.json` | Encrypted secrets store (AES-256-GCM) |
| `~/.orchestr8/plugins.json` | Plugin registry |
| `~/.orchestr8/plugins/` | Plugin manifest directory (discovery source) |
| `~/.orchestr8/backups/` | Backup directory |

---

## 🌍 Environment Variables

| Variable | Description | Default |
|---|---|---|
| `ORCHESTR8_SECRET_KEY` | Encryption key for secrets. Enables AES-256-GCM when set. | Built-in dev key (XOR obfuscation, NOT secure) |

### Setting the Secret Key

```bash
# For the session
export ORCHESTR8_SECRET_KEY="my-production-secret-key-at-least-16-chars"

# In your shell profile
echo 'export ORCHESTR8_SECRET_KEY="your-key-here"' >> ~/.bashrc

# Per-command
ORCHESTR8_SECRET_KEY="key" orchestr8 secrets set db-creds password "val"
```

---

## 🚩 Common Flags

### Global Flags (apply to all commands)

| Flag | Short | Default | Description |
|---|---|---|---|
| `--spec` | `-s` | `workload.yaml` | Workload spec file path |
| `--verbose` | `-v` | off | Enable verbose logging |
| `--quiet` | `-q` | off | Suppress output except errors |
| `--json` | -- | off | Output as JSON |
| `--output` | `-o` | `table` | Format: `table`, `json`, `yaml`, `wide` |
| `--yes` | `-y` | off | Skip confirmation prompts (CI/automation) |
| `--dry-run` | -- | off | Show what would happen without executing |
| `--skip-policy` | -- | off | Skip policy checks on deploy |

---

## 🎼 Compose Commands

```bash
orchestr8 compose validate                          # Validate compose file
orchestr8 compose validate ./stack.yaml             # Validate specific file
orchestr8 compose up                                # Deploy all workloads in dependency order
orchestr8 compose up --runtime kube                 # Override runtime for all workloads
orchestr8 compose up --dry-run                      # Preview deployment plan
orchestr8 compose down                              # Stop all workloads (reverse order)
```

**Default compose file:** `orchestr8-compose.yaml`

### Compose File Quick Reference

```yaml
version: "1"
workloads:
  db:
    spec: ./db.yaml
    runtime: container          # optional runtime override
    env:                        # optional env injection
      POSTGRES_DB: myapp
  api:
    spec: ./api.yaml
    depends_on: [db]            # dependency ordering
```

---

## 🔌 Plugin Commands

```bash
orchestr8 plugin list                               # List registered plugins
orchestr8 plugin discover                           # Scan ~/.orchestr8/plugins/
orchestr8 plugin register ./manifest.json           # Register from manifest file
orchestr8 plugin remove my-runtime                  # Unregister plugin by name
```

---

## 🏥 Health Commands

```bash
orchestr8 health my-app                             # Show health timeline (last 20)
orchestr8 health my-app --last 50                   # Show last 50 records
orchestr8 health my-app --summary                   # Summary only (uptime %, restarts)
orchestr8 orchestrate health-check                  # One-shot health check (all workloads)
orchestr8 orchestrate watch                         # Continuous monitoring (30s default)
orchestr8 orchestrate watch --interval 10           # Custom interval (seconds)
orchestr8 orchestrate status                        # All workload health statuses
orchestr8 orchestrate summary                       # Aggregated health summary
```

---

## 🔐 Secrets Commands

```bash
orchestr8 secrets create db-creds                   # Create secret (default namespace)
orchestr8 secrets create db-creds --namespace prod  # Create with namespace
orchestr8 secrets set db-creds password "s3cret"    # Set (or update) key-value
orchestr8 secrets get db-creds password             # Get decrypted value
orchestr8 secrets list                              # List all secrets (no values shown)
orchestr8 secrets audit                             # Check rotation status
```

---

## 🧠 AI and Intelligence

```bash
orchestr8 recommend                                 # AI runtime recommendation with scoring
orchestr8 profile                                   # Resource profiling and waste detection
orchestr8 analyze-logs my-app                       # Log anomaly detection
orchestr8 migration-advice my-app kube              # Migration risk assessment
orchestr8 scaling-advice                            # Predictive scaling recommendation
orchestr8 drift my-app                              # Configuration drift detection
orchestr8 drift my-app --reconcile                  # Auto-reconcile detected drift
orchestr8 policy-check                              # Check spec against policies
orchestr8 policy-check --policy development         # Use development policy set
```

---

## 📚 More Documentation

| Guide | Path |
|---|---|
| Compose Feature | [`docs/features/compose.md`](../features/compose.md) |
| Plugin System | [`docs/features/plugins.md`](../features/plugins.md) |
| Health Monitoring | [`docs/features/health-monitoring.md`](../features/health-monitoring.md) |
| Security Features | [`docs/features/security.md`](../features/security.md) |
| REST API Reference | [`docs/reference/api/API-Reference.md`](../reference/api/API-Reference.md) |
