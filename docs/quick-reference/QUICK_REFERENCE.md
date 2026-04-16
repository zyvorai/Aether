# ⚡ aether Quick Reference Card

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
aether validate                      # Validate workload spec
aether build                         # Build container image
aether run                           # Deploy (auto-select runtime)
aether run --runtime kube            # Deploy to specific runtime
aether status my-app                 # Check status
aether logs my-app                   # View logs
aether logs my-app --follow          # Stream logs in real-time
aether stop my-app                   # Stop workload
aether delete my-app                 # Delete workload
aether list                          # List all workloads
```

### Migration

```bash
aether migrate my-app kube                     # Migrate (default: blue-green)
aether migrate my-app kube -s immediate        # Immediate migration
aether migrate my-app kube -s rolling          # Rolling migration
aether migrate my-app metal --no-rollback      # Disable automatic rollback
aether migrate my-app kube --no-validation     # Skip validation delay
```

### Custom Spec File

```bash
aether -s ./my-app.yaml validate              # Use custom spec file
aether -s ./my-app.yaml run --runtime podman   # Deploy custom spec
```

### Dashboard and API

```bash
aether tui                           # Launch interactive TUI dashboard
aether serve                         # Start REST API server (localhost:5090)
aether serve --host 0.0.0.0 -p 3000 # Bind to all interfaces, custom port
```

### Backup and Restore

```bash
aether backup                        # Create backup (auto-named)
aether backup -n pre-deploy          # Named backup
aether backup -d "Before migration"  # Backup with description
aether list-backups                  # List all backups
aether restore ./backup.json         # Restore from backup
aether restore ./backup.json --merge # Merge with existing state
```

### Batch Operations

```bash
aether deploy ./specs/               # Deploy all YAML in directory
aether deploy ./specs/ --fail-fast   # Stop on first failure
aether deploy ./specs/ --dry-run     # Preview deployment plan
aether deploy ./specs/ -r kube       # Override runtime for all
```

### Advanced Operations

```bash
aether exec my-app                   # Shell into workload (/bin/sh)
aether exec my-app "ls -la"          # Run command in workload
aether port-forward my-app 8080:80   # Forward local:remote ports
aether watch                         # Auto-redeploy on spec changes
aether diff my-app                   # Compare spec vs live state
aether rollback my-app               # Rollback to latest snapshot
aether compare                       # Compare runtimes for spec
aether init                          # First-time setup wizard
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
aether list -o json                  # JSON output
aether list -o yaml                  # YAML output
aether list -o wide                  # Wide table with extra columns
aether list --quiet                  # Errors only
aether list --json                   # Shorthand JSON
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
~/.aether/config.yaml
```

### Config Commands

```bash
aether config --show                 # Display current configuration
aether config --init                 # Create default config file
```

### State and Data Files

| File | Purpose |
|---|---|
| `~/.aether/config.yaml` | Configuration file |
| `~/.aether/state.json` | Workload state database (file-locked, atomic writes) |
| `~/.aether/health.json` | Health check history (bounded ring buffer, max 1000) |
| `~/.aether/secrets.json` | Encrypted secrets store (AES-256-GCM) |
| `~/.aether/plugins.json` | Plugin registry |
| `~/.aether/plugins/` | Plugin manifest directory (discovery source) |
| `~/.aether/backups/` | Backup directory |

---

## 🌍 Environment Variables

| Variable | Description | Default |
|---|---|---|
| `AETHER_NAMESPACE` | Kubernetes namespace for kube-based runtimes. Overridden by `-n` flag. | `default` |
| `AETHER_SECRET_KEY` | Encryption key for secrets. Enables AES-256-GCM when set. | Machine-derived key (logged warning) |
| `AETHER_API_KEY` | Bearer token for API authentication (backward-compat fallback when RBAC keys exist). | Unset (API is public) |
| `AETHER_LOG_FORMAT` | Set to `json` for structured JSON log output. | Unset (human-readable) |

### Setting the Secret Key

```bash
# For the session
export AETHER_SECRET_KEY="my-production-secret-key-at-least-16-chars"

# In your shell profile
echo 'export AETHER_SECRET_KEY="your-key-here"' >> ~/.bashrc

# Per-command
AETHER_SECRET_KEY="key" aether secrets set db-creds password "val"
```

---

## 🚩 Common Flags

### Global Flags (apply to all commands)

| Flag | Short | Default | Description |
|---|---|---|---|
| `--spec` | `-s` | `workload.yaml` | Workload spec file path |
| `--namespace` | `-n` | -- | Kubernetes namespace override (or `AETHER_NAMESPACE` env var) |
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
aether compose validate                          # Validate compose file
aether compose validate ./stack.yaml             # Validate specific file
aether compose up                                # Deploy all workloads in dependency order
aether compose up --runtime kube                 # Override runtime for all workloads
aether compose up --dry-run                      # Preview deployment plan
aether compose down                              # Stop all workloads (reverse order)
```

**Default compose file:** `aether-compose.yaml`

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
aether plugin list                               # List registered plugins
aether plugin discover                           # Scan ~/.aether/plugins/
aether plugin register ./manifest.json           # Register from manifest file
aether plugin remove my-runtime                  # Unregister plugin by name
```

---

## 🏥 Health Commands

```bash
aether health my-app                             # Show health timeline (last 20)
aether health my-app --last 50                   # Show last 50 records
aether health my-app --summary                   # Summary only (uptime %, restarts)
aether orchestrate health-check                  # One-shot health check (all workloads)
aether orchestrate watch                         # Continuous monitoring (30s default)
aether orchestrate watch --interval 10           # Custom interval (seconds)
aether orchestrate status                        # All workload health statuses
aether orchestrate summary                       # Aggregated health summary
```

---

## 🔐 Secrets Commands

```bash
aether secrets create db-creds                   # Create secret (default namespace)
aether secrets create db-creds --namespace prod  # Create with namespace
aether secrets set db-creds password "s3cret"    # Set (or update) key-value
aether secrets get db-creds password             # Get decrypted value
aether secrets list                              # List all secrets (no values shown)
aether secrets audit                             # Check rotation status
```

---

## 🛂 RBAC Commands

```bash
# Managed via REST API (requires Admin role)
curl http://localhost:5090/api/rbac/keys                          # List RBAC keys
curl -X POST http://localhost:5090/api/rbac/keys \
  -d '{"name":"ci","role":"operator"}'                            # Create key
curl -X POST http://localhost:5090/api/rbac/keys/revoke \
  -d '{"name":"ci"}'                                              # Revoke key
```

---

## 🧠 AI and Intelligence

```bash
aether recommend                                 # AI runtime recommendation with scoring
aether profile                                   # Resource profiling and waste detection
aether analyze-logs my-app                       # Log anomaly detection
aether migration-advice my-app kube              # Migration risk assessment
aether scaling-advice                            # Predictive scaling recommendation
aether intent                                    # Evaluate intent goals and scoring
aether drift my-app                              # Configuration drift detection
aether drift my-app --reconcile                  # Auto-reconcile detected drift
aether policy-check                              # Check spec against policies
aether policy-check --policy development         # Use development policy set
```

---

## 🌐 Web Dashboard Shortcuts

| Shortcut | Action |
|---|---|
| `r` | Refresh current page |
| `?` | Open Command Palette |
| `Cmd+K` / `Ctrl+K` | Open Command Palette |

### Dashboard Features

- **SSE real-time updates** -- mutations push events instantly (no polling)
- **WorkloadDetail** -- click workload name for tabbed detail panel (Overview/Logs/Drift/Scoring)
- **LogViewer** -- auto-poll, follow mode, filter, line numbers, color-coded levels, copy-to-clipboard
- **Command Palette** -- fuzzy search across pages, workloads, and actions
- **Intent Debugger** -- SVG radar chart for AI scoring visualization
- **Connection indicator** -- green/red dot showing SSE connection status

---

## 📚 More Documentation

| Guide | Path |
|---|---|
| Compose Feature | [`docs/features/compose.md`](../features/compose.md) |
| Plugin System | [`docs/features/plugins.md`](../features/plugins.md) |
| Health Monitoring | [`docs/features/health-monitoring.md`](../features/health-monitoring.md) |
| Security Features | [`docs/features/security.md`](../features/security.md) |
| REST API Reference | [`docs/reference/api/API-Reference.md`](../reference/api/API-Reference.md) |
