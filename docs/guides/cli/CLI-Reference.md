# CLI Reference

Complete command reference for Orchestr8.

---

## Global Options

| Flag | Description |
|------|-------------|
| `--spec <file>` | Workload spec file (default: `workload.yaml`) |
| `-v, --verbose` | Enable debug logging |
| `-q, --quiet` | Suppress output except errors |
| `--json` | JSON output mode |
| `-o, --output <fmt>` | Output format: `table`, `json`, `yaml`, `wide` |
| `--dry-run` | Preview without executing |
| `-y, --yes` | Skip confirmation prompts |
| `--skip-policy` | Bypass policy checks on deploy |

---

## Workload Lifecycle

### `validate`
Validate a workload specification.
```bash
orchestr8 validate --spec app.yaml
```

### `build`
Build the container image for a workload.
```bash
orchestr8 build --spec app.yaml
```

### `run`
Deploy a workload (interactive runtime selector if no `--runtime`).
```bash
orchestr8 run --spec app.yaml [--runtime podman|kube|kubevirt|metal]
```

### `stop`
Stop a running workload.
```bash
orchestr8 stop <name>
```

### `status`
Get workload status. Also records a health check.
```bash
orchestr8 status <name>
```

### `logs`
View workload logs.
```bash
orchestr8 logs <name> [--follow]
```

### `delete`
Delete a workload and cascade-clean all subsystem data.
```bash
orchestr8 delete <name>
```

### `list`
List all deployed workloads.
```bash
orchestr8 list [--output wide|json|yaml]
```

---

## Migration

### `migrate`
Migrate a workload to a different runtime.
```bash
orchestr8 migrate <name> <target> [--strategy immediate|blue-green|rolling] [--no-validation] [--no-rollback]
```

### `rollback`
Rollback to previous deployment state.
```bash
orchestr8 rollback <name>
```

---

## Developer Workflow

### `exec`
Execute a command inside a running workload.
```bash
orchestr8 exec <name> [command] [-i] [-t timeout]
```

### `port-forward`
Forward local ports to a workload.
```bash
orchestr8 port-forward <name> <local:remote> [-t timeout]
```

### `watch`
Watch spec file and auto-redeploy on changes.
```bash
orchestr8 watch [--runtime podman]
```

### `compare`
Compare workload suitability across all runtimes.
```bash
orchestr8 compare
```

### `init`
First-time setup wizard.
```bash
orchestr8 init
```

### `health`
View workload health history and uptime.
```bash
orchestr8 health <name> [--last 20] [--summary]
```

---

## Compose

### `compose validate`
Validate a compose file.
```bash
orchestr8 compose validate [file]
```

### `compose up`
Deploy all workloads in dependency order.
```bash
orchestr8 compose up [file] [--runtime kube] [--dry-run]
```

### `compose down`
Stop all workloads in reverse dependency order.
```bash
orchestr8 compose down [file]
```

---

## Plugins

### `plugin list`
List registered plugins.
```bash
orchestr8 plugin list
```

### `plugin discover`
Scan `~/.orchestr8/plugins/` for plugin manifests.
```bash
orchestr8 plugin discover
```

### `plugin register`
Register a plugin from a manifest file.
```bash
orchestr8 plugin register <manifest.json>
```

### `plugin remove`
Unregister a plugin.
```bash
orchestr8 plugin remove <name>
```

---

## Operations

### `deploy`
Batch-deploy all specs in a directory.
```bash
orchestr8 deploy <dir> [--runtime kube] [--fail-fast] [--dry-run]
```

### `diff`
Compare live state against spec.
```bash
orchestr8 diff <name>
```

### `drift`
Detect and optionally reconcile configuration drift.
```bash
orchestr8 drift <name> [--reconcile]
```

### `backup` / `restore` / `list-backups`
```bash
orchestr8 backup [name] [--description "..."]
orchestr8 restore <file> [--merge]
orchestr8 list-backups
```

---

## Governance

### `policy-check`
Evaluate workload against policy rules.
```bash
orchestr8 policy-check --spec app.yaml [--policy production|development|<file>]
```

### `secrets`
Manage encrypted secrets.
```bash
orchestr8 secrets create <name> [--namespace default]
orchestr8 secrets set <name> <key> <value>
orchestr8 secrets get <name> <key>
orchestr8 secrets list
orchestr8 secrets audit <name>
```

### `audit`
View audit trail.
```bash
orchestr8 audit [--last 50] [--workload name] [--summary]
```

### `events`
View event log.
```bash
orchestr8 events [--last 50] [--severity warning] [--summary]
```

---

## Advanced

| Command | Description |
|---------|-------------|
| `cost` | Estimate deployment cost |
| `recommend` | AI runtime recommendation |
| `profile` | Workload profiling |
| `analyze-logs` | AI log analysis |
| `migration-advice` | Migration strategy recommendation |
| `scaling-advice` | Scaling recommendations |
| `template` | Generate workload from template |
| `sla` | SLA management |
| `schedule` | Workload scheduling |
| `orchestrate` | Health orchestration and watch loop |
| `affinity` | Runtime affinity learning |
| `webhook` | Webhook management (add/remove/list/test/queue/flush) |
| `env` | Environment management |
| `config` | Configuration management |

---

## UI & Utilities

| Command | Description |
|---------|-------------|
| `tui` | Launch interactive TUI dashboard |
| `serve` | Start REST API server |
| `completions` | Generate shell completions |
| `help-all` | Show complete command reference with examples |
