# 📖 Aether CLI Reference

> **Version:** See `aether --version`
> **License:** Proprietary HyperSDK

Aether is the Universal Runtime Control Plane. One spec, three runtimes:
Podman, Kubernetes, and KubeVirt.

---

## 📑 Table of Contents

- [Global Options](#-global-options)
- [Workload Lifecycle](#-workload-lifecycle)
  - [validate](#validate)
  - [build](#build)
  - [run](#run)
  - [stop](#stop)
  - [status](#status)
  - [logs](#logs)
  - [delete](#delete)
  - [list](#list)
- [Migration](#-migration)
  - [migrate](#migrate)
  - [rollback](#rollback)
- [Developer Workflow](#-developer-workflow)
  - [exec](#exec)
  - [port-forward](#port-forward)
  - [watch](#watch)
  - [compare](#compare)
  - [init](#init)
  - [health](#health)
  - [diff](#diff)
- [Compose](#-compose)
  - [compose validate](#compose-validate)
  - [compose up](#compose-up)
  - [compose down](#compose-down)
- [Plugins](#-plugins)
  - [plugin list](#plugin-list)
  - [plugin discover](#plugin-discover)
  - [plugin register](#plugin-register)
  - [plugin remove](#plugin-remove)
- [Operations](#-operations)
  - [backup](#backup)
  - [restore](#restore)
  - [list-backups](#list-backups)
  - [deploy](#deploy)
  - [drift](#drift)
- [Governance](#-governance)
  - [policy-check](#policy-check)
  - [secrets](#secrets)
  - [sla](#sla)
  - [audit](#audit)
  - [events](#events)
- [Advanced / AI](#-advanced--ai)
  - [cost](#cost)
  - [recommend](#recommend)
  - [profile](#profile)
  - [analyze-logs](#analyze-logs)
  - [migration-advice](#migration-advice)
  - [scaling-advice](#scaling-advice)
- [Orchestration](#-orchestration)
  - [orchestrate](#orchestrate)
  - [schedule](#schedule)
  - [affinity](#affinity)
  - [webhook](#webhook)
  - [env](#env)
  - [template](#template)
  - [deps](#deps)
- [UI and Utilities](#-ui-and-utilities)
  - [tui](#tui)
  - [serve](#serve)
  - [help-all](#help-all)
  - [completions](#completions)
  - [config](#config)
  - [metrics](#metrics)

---

## 🌍 Global Options

These flags apply to **every** command:

| Flag              | Short | Default          | Description                                      |
|-------------------|-------|------------------|--------------------------------------------------|
| `--spec <FILE>`   | `-s`  | `workload.yaml`  | Workload specification file                      |
| `--namespace <NS>`| `-n`  |                  | Kubernetes namespace override (also `AETHER_NAMESPACE` env var) |
| `--verbose`       | `-v`  |                  | Enable debug-level logging                       |
| `--quiet`         | `-q`  |                  | Suppress all output except errors                |
| `--json`          |       |                  | Output results as JSON (conflicts with `--quiet`)|
| `--output <FMT>`  | `-o`  | `table`          | Output format: `table`, `json`, `yaml`, `wide`   |
| `--yes`           | `-y`  |                  | Skip confirmation prompts (CI/automation)        |
| `--dry-run`       |       |                  | Show what would happen without executing         |
| `--skip-policy`   |       |                  | Skip policy checks on deploy (use with caution)  |

**Namespace resolution order:** `--namespace` flag > `AETHER_NAMESPACE` env var > `"default"`.

```bash
# Deploy to a specific namespace
aether -n staging run --runtime kube

# Or use the environment variable
export AETHER_NAMESPACE=production
aether run --runtime kube
```

---

## 📦 Workload Lifecycle

### validate

Validate a workload specification file against the schema.

```
aether [OPTIONS] validate
```

**Examples:**

```bash
aether validate
aether --spec my-app.yaml validate
aether --output json validate
```

---

### build

Build a container image from the workload spec. The runtime is selected
automatically by the decision engine.

```
aether [OPTIONS] build
```

**Examples:**

```bash
aether build
aether --spec api.yaml build
```

---

### run

Deploy a workload instance. Builds the image, starts the instance, and persists
state. Shows an interactive runtime selector unless `--runtime` is provided.

```
aether [OPTIONS] run [--runtime <RUNTIME>]
```

| Flag               | Short | Description                                        |
|--------------------|-------|----------------------------------------------------|
| `--runtime <NAME>` | `-r`  | Override runtime: `podman`, `kube`, `kubevirt`, `metal` |

**Examples:**

```bash
aether run
aether run --runtime podman
aether --dry-run run
aether --skip-policy run --runtime kube
```

---

### stop

Stop a running workload instance. The state is preserved for restart.

```
aether [OPTIONS] stop <NAME>
```

**Examples:**

```bash
aether stop hello-web
aether --dry-run stop hello-web
```

---

### status

Get the current status of a workload instance.

```
aether [OPTIONS] status <NAME>
```

**Examples:**

```bash
aether status hello-web
aether --output json status hello-web
aether --output yaml status hello-web
```

---

### logs

View logs from a workload instance.

```
aether [OPTIONS] logs <NAME> [--follow]
```

| Flag       | Short | Description                |
|------------|-------|----------------------------|
| `--follow` | `-f`  | Follow log output in real time |

**Examples:**

```bash
aether logs hello-web
aether logs hello-web --follow
```

---

### delete

Delete a workload instance and remove it from state.

```
aether [OPTIONS] delete <NAME>
```

**Examples:**

```bash
aether delete hello-web
aether --yes delete hello-web
aether --dry-run delete hello-web
```

---

### list

List all tracked workload instances.

```
aether [OPTIONS] list
```

**Examples:**

```bash
aether list
aether --output wide list
aether --json list
```

---

## 🔀 Migration

### migrate

Migrate a workload from its current runtime to a different one.

```
aether [OPTIONS] migrate <NAME> <TARGET> [--strategy <STRATEGY>] [--no-validation] [--no-rollback]
```

| Flag               | Default      | Description                              |
|--------------------|--------------|------------------------------------------|
| `--strategy`       | `blue-green` | Strategy: `immediate`, `blue-green`, `rolling` |
| `--no-validation`  |              | Skip post-migration validation delay     |
| `--no-rollback`    |              | Disable automatic rollback on failure    |

**Strategies:**

| Strategy     | Downtime | Behavior                                                |
|--------------|----------|---------------------------------------------------------|
| `immediate`  | Yes      | Stop source, build + deploy target, validate            |
| `blue-green` | No       | Deploy target alongside source, switch traffic, cleanup |
| `rolling`    | No       | Deploy target, validate with retries, gradual traffic shift (25/50/75/100%) |

**Examples:**

```bash
aether migrate hello-web kube
aether migrate hello-web kube --strategy rolling
aether migrate hello-web podman --strategy immediate --no-rollback
aether --dry-run migrate hello-web kubevirt --strategy blue-green
```

---

### rollback

Rollback a workload to its latest pre-deploy snapshot.

```
aether [OPTIONS] rollback <NAME>
```

**Examples:**

```bash
aether rollback hello-web
```

---

## 🛠 Developer Workflow

### exec

Execute a command inside a running workload container.

```
aether [OPTIONS] exec <NAME> [COMMAND] [--interactive] [--timeout <SECS>]
```

| Flag              | Short | Default   | Description                      |
|-------------------|-------|-----------|----------------------------------|
| `COMMAND`         |       | `/bin/sh` | Command to execute               |
| `--interactive`   | `-i`  |           | Pass stdin to the container      |
| `--timeout`       | `-t`  | `0`       | Timeout in seconds (0 = none)    |

**Examples:**

```bash
aether exec hello-web
aether exec hello-web "ls -la /app" --timeout 30
aether exec hello-web "/bin/bash" --interactive
```

---

### port-forward

Forward local ports to a running workload.

```
aether [OPTIONS] port-forward <NAME> <PORTS> [--timeout <SECS>]
```

| Flag          | Short | Default | Description                      |
|---------------|-------|---------|----------------------------------|
| `PORTS`       |       |         | Port mapping `local:remote`      |
| `--timeout`   | `-t`  | `0`     | Timeout in seconds (0 = none)    |

**Examples:**

```bash
aether port-forward hello-web 8080:80
aether port-forward hello-web 3000:3000 --timeout 300
```

---

### watch

Watch the spec file and auto-redeploy on changes.

```
aether [OPTIONS] watch [--runtime <RUNTIME>]
```

**Examples:**

```bash
aether watch
aether --spec api.yaml watch --runtime podman
```

---

### compare

Compare a workload across all three runtimes (cost, capabilities, limitations).

```
aether [OPTIONS] compare
```

**Examples:**

```bash
aether compare
aether --output json compare
```

---

### init

Run the first-time setup wizard. Detects runtimes, creates config, ensures directories.

```
aether init
```

---

### health

View health history and uptime statistics for a workload.

```
aether [OPTIONS] health <NAME> [--last <N>] [--summary]
```

| Flag        | Short | Default | Description                       |
|-------------|-------|---------|-----------------------------------|
| `--last`    | `-l`  | `20`    | Show last N health records        |
| `--summary` |       |         | Show summary statistics only      |

**Examples:**

```bash
aether health hello-web
aether health hello-web --last 50
aether health hello-web --summary
```

---

### diff

Three-way diff: spec file vs stored state vs live runtime.

```
aether [OPTIONS] diff <NAME>
```

**Examples:**

```bash
aether diff hello-web
aether --output json diff hello-web
```

---

## 🗂 Compose

### compose validate

Validate a compose file for structural correctness, dependency cycles, and
missing references.

```
aether compose validate [FILE]
```

**Examples:**

```bash
aether compose validate
aether compose validate ./deploy/aether-compose.yaml
```

---

### compose up

Deploy all workloads defined in a compose file, in dependency order.

```
aether compose up [FILE] [--runtime <RUNTIME>] [--dry-run]
```

| Flag          | Short | Description                              |
|---------------|-------|------------------------------------------|
| `--runtime`   | `-r`  | Override runtime for all workloads       |
| `--dry-run`   |       | Show deployment plan without executing   |

**Examples:**

```bash
aether compose up
aether compose up --runtime podman
aether compose up --dry-run
aether compose up ./deploy/aether-compose.yaml
```

---

### compose down

Stop and remove all workloads from a compose file, in reverse dependency order.

```
aether compose down [FILE]
```

**Examples:**

```bash
aether compose down
aether compose down ./deploy/aether-compose.yaml
```

---

## 🧩 Plugins

### plugin list

List all registered plugins.

```
aether plugin list
```

---

### plugin discover

Scan `~/.aether/plugins/` for `*.json` manifest files and register them.

```
aether plugin discover
```

---

### plugin register

Register a plugin from a manifest file.

```
aether plugin register <MANIFEST>
```

**Examples:**

```bash
aether plugin register ./wasm-runtime.json
```

---

### plugin remove

Unregister a plugin by name.

```
aether plugin remove <NAME>
```

**Examples:**

```bash
aether plugin remove wasm-runtime
```

---

## 🔧 Operations

### backup

Create a backup of the current workload state.

```
aether [OPTIONS] backup [--name <NAME>] [--description <DESC>]
```

| Flag              | Short | Description                                    |
|-------------------|-------|------------------------------------------------|
| `--name`          | `-n`  | Backup name (auto-generated if not provided)   |
| `--description`   | `-d`  | Human-readable description                     |

**Examples:**

```bash
aether backup
aether backup --name pre-migration --description "Before kube migration"
```

---

### restore

Restore workload state from a backup file.

```
aether [OPTIONS] restore <BACKUP> [--merge]
```

| Flag      | Short | Description                                     |
|-----------|-------|-------------------------------------------------|
| `--merge` | `-m`  | Merge with existing state instead of replacing  |

**Examples:**

```bash
aether restore ~/.aether/backups/backup-20260411-103000.json
aether restore backup.json --merge
```

---

### list-backups

List all available backup files.

```
aether list-backups
```

---

### deploy

Deploy all workload specs from a directory.

```
aether [OPTIONS] deploy <DIR> [--runtime <RUNTIME>] [--fail-fast] [--dry-run]
```

| Flag           | Short | Description                              |
|----------------|-------|------------------------------------------|
| `--runtime`    | `-r`  | Override runtime for all workloads       |
| `--fail-fast`  |       | Stop on first failure                    |
| `--dry-run`    |       | Show deployment plan without executing   |

**Examples:**

```bash
aether deploy ./specs/
aether deploy ./specs/ --runtime podman --fail-fast
aether deploy ./specs/ --dry-run
```

---

### drift

Detect configuration drift between the desired spec and live state.

```
aether [OPTIONS] drift <NAME> [--reconcile]
```

| Flag           | Description                                     |
|----------------|-------------------------------------------------|
| `--reconcile`  | Automatically reconcile detected drift          |

**Examples:**

```bash
aether drift hello-web
aether drift hello-web --reconcile
```

---

## 🏛 Governance

### policy-check

Evaluate a workload spec against deployment policies.

```
aether [OPTIONS] policy-check [--policy <POLICY>]
```

| Flag        | Short | Default      | Description                                     |
|-------------|-------|--------------|-------------------------------------------------|
| `--policy`  | `-p`  | `production` | Policy set: `production`, `development`, or file path |

**Examples:**

```bash
aether policy-check
aether policy-check --policy development
aether policy-check --policy ./custom-policies.yaml
```

---

### secrets

Manage encrypted secrets (AES-256-GCM when `AETHER_SECRET_KEY` is set).

```
aether secrets <SUBCOMMAND>
```

| Subcommand                              | Description                          |
|-----------------------------------------|--------------------------------------|
| `create <NAME> [--namespace <NS>]`      | Create a new secret                  |
| `set <SECRET> <KEY> <VALUE>`            | Set a key-value pair                 |
| `get <SECRET> <KEY>`                    | Get a decrypted value                |
| `list`                                  | List all secrets (no values shown)   |
| `audit`                                 | Check rotation status                |

**Examples:**

```bash
aether secrets create db-creds --namespace production
aether secrets set db-creds password "s3cret"
aether secrets get db-creds password
aether secrets list
aether secrets audit
```

---

### sla

SLA compliance monitoring.

```
aether sla <SUBCOMMAND>
```

| Subcommand                                                                   | Description              |
|------------------------------------------------------------------------------|--------------------------|
| `add <WORKLOAD> [--tier <TIER>]`                                             | Add an SLA target        |
| `check <WORKLOAD> --uptime <PCT> [--latency <MS>] [--error-rate <PCT>] [--restarts <N>]` | Check compliance |
| `list`                                                                       | List all SLA targets     |

**Tiers:** `standard`, `high-availability`, `best-effort`

**Examples:**

```bash
aether sla add hello-web --tier high-availability
aether sla check hello-web --uptime 99.9 --latency 50 --error-rate 0.01
aether sla list
```

---

### audit

View the audit trail of workload operations.

```
aether [OPTIONS] audit [--last <N>] [--workload <NAME>] [--summary]
```

| Flag           | Short | Default | Description                       |
|----------------|-------|---------|-----------------------------------|
| `--last`       | `-l`  | `20`    | Show last N events                |
| `--workload`   | `-w`  |         | Filter by workload name           |
| `--summary`    |       |         | Show summary only                 |

**Examples:**

```bash
aether audit
aether audit --last 50 --workload hello-web
aether audit --summary
```

---

### events

View and manage the event stream.

```
aether [OPTIONS] events [--last <N>] [--severity <LEVEL>] [--summary]
```

| Flag          | Default | Description                                            |
|---------------|---------|--------------------------------------------------------|
| `--last`      | `20`    | Show last N events                                     |
| `--severity`  |         | Filter: `info`, `warning`, `error`, `critical`        |
| `--summary`   |         | Show summary only                                      |

**Examples:**

```bash
aether events
aether events --last 100 --severity error
aether events --summary
```

---

## 🤖 Advanced / AI

### cost

Estimate workload costs across cloud providers.

```
aether [OPTIONS] cost [--provider <PROVIDER>]
```

| Flag          | Short | Default | Description                                       |
|---------------|-------|---------|---------------------------------------------------|
| `--provider`  | `-c`  | `all`   | `aws`, `azure`, `gcp`, `digitalocean`, `linode`, `all` |

**Examples:**

```bash
aether cost
aether cost --provider aws
aether --output json cost --provider all
```

---

### recommend

AI-powered runtime recommendation with scoring.

```
aether [OPTIONS] recommend [--runtime <RUNTIME>]
```

**Examples:**

```bash
aether recommend
aether recommend --runtime kube
```

---

### profile

Profile a workload and show optimization recommendations.

```
aether [OPTIONS] profile [--name <NAME>]
```

**Examples:**

```bash
aether profile
aether profile --name hello-web
```

---

### analyze-logs

Analyze workload logs for anomalies and patterns.

```
aether [OPTIONS] analyze-logs <NAME>
```

**Examples:**

```bash
aether analyze-logs hello-web
```

---

### migration-advice

Get AI-powered migration advice for a workload.

```
aether [OPTIONS] migration-advice <NAME> <TARGET>
```

**Examples:**

```bash
aether migration-advice hello-web kube
aether migration-advice api-service kubevirt
```

---

### scaling-advice

Show predictive scaling recommendations based on historical data.

```
aether [OPTIONS] scaling-advice
```

**Examples:**

```bash
aether scaling-advice
aether --output json scaling-advice
```

---

## 🎛 Orchestration

### orchestrate

Health-aware orchestration with circuit breakers and rolling updates.

```
aether orchestrate <SUBCOMMAND>
```

| Subcommand                                       | Description                                   |
|--------------------------------------------------|-----------------------------------------------|
| `register <NAME> [--runtime <RT>]`               | Register a workload for health monitoring     |
| `status`                                         | Show health status of all workloads           |
| `summary`                                        | Show health summary                           |
| `rolling-update <NAME> [--replicas <N>]`         | Simulate a rolling update                     |
| `reset-circuit <NAME>`                           | Reset circuit breaker for a workload          |
| `health-check`                                   | Run a single round of health checks           |
| `watch [--interval <SECS>]`                      | Continuously monitor health + evaluate alert rules (default: 30s) |

**Examples:**

```bash
aether orchestrate register api --runtime kubernetes
aether orchestrate status
aether orchestrate watch --interval 60
aether orchestrate rolling-update api --replicas 5
aether orchestrate reset-circuit api
aether orchestrate health-check
```

---

### schedule

Workload scheduling and placement optimization.

```
aether schedule <SUBCOMMAND>
```

| Subcommand                                                                  | Description                  |
|-----------------------------------------------------------------------------|------------------------------|
| `place <NAME> [--cpu <N>] [--memory <MB>] [--strategy <S>] [--prefer <RT>]` | Schedule a workload         |
| `utilization`                                                               | Show runtime utilization     |
| `optimize`                                                                  | Get optimization suggestions |
| `placements`                                                                | Show current placements      |

**Strategies:** `balanced`, `cost`, `performance`, `bin-packing`

**Examples:**

```bash
aether schedule place api --cpu 4 --memory 8192 --strategy cost
aether schedule utilization
aether schedule optimize
aether schedule placements
```

---

### affinity

Runtime affinity learning and recommendations.

```
aether affinity <SUBCOMMAND>
```

| Subcommand             | Description                              |
|------------------------|------------------------------------------|
| `recommend <CLASS>`    | Show recommendations for a workload class |
| `matrix`               | Show compatibility matrix                |
| `stats`                | Show learning statistics                 |

**Workload classes:** `web-service`, `api-backend`, `database`, `cache`,
`batch-job`, `ml-training`, `worker`, `microservice`

**Examples:**

```bash
aether affinity recommend web-service
aether affinity matrix
aether affinity stats
```

---

### webhook

Manage webhook notification channels with retry queue.

```
aether webhook <SUBCOMMAND>
```

| Subcommand                                                              | Description                        |
|-------------------------------------------------------------------------|------------------------------------|
| `add <NAME> <URL> [--method <M>] [--severity <S>]`                     | Add a notification channel         |
| `remove <NAME>`                                                        | Remove a channel                   |
| `list`                                                                  | List all channels                  |
| `test <NAME>`                                                          | Send a test notification           |
| `queue`                                                                | Show pending deliveries            |
| `flush`                                                                | Force-retry all queued webhooks    |

**Examples:**

```bash
aether webhook add slack "https://hooks.slack.com/..." --severity warning
aether webhook test slack
aether webhook queue
aether webhook flush
aether webhook list
aether webhook remove slack
```

---

### env

Manage deployment environments.

```
aether env <SUBCOMMAND>
```

| Subcommand                                  | Description                        |
|---------------------------------------------|------------------------------------|
| `create <NAME> [--tier <TIER>]`             | Create a new environment           |
| `list`                                      | List environments                  |
| `promote <WORKLOAD> <FROM> <TO>`            | Promote workload between envs      |
| `parity <ENV1> <ENV2>`                      | Check parity between environments  |

**Tiers:** `development`, `staging`, `production`

**Examples:**

```bash
aether env create staging --tier staging
aether env list
aether env promote hello-web staging production
aether env parity staging production
```

---

### template

Generate a workload spec from a built-in template.

```
aether template <NAME> [--workload-name <WN>] [--owner <O>] [--project <P>] [--registry <R>] [--output <FILE>] [--list]
```

**Templates:** `web-app`, `rest-api`, `database`, `cache`, `worker`, `cron-job`,
`ml-training`, `microservice`

| Flag                | Default          | Description                     |
|---------------------|------------------|---------------------------------|
| `--workload-name`   |                  | Workload name in generated spec |
| `--owner`           | `team`           | Owner field                     |
| `--project`         | `default`        | Project field                   |
| `--registry`        | `ghcr.io/org`    | Container registry              |
| `--output`          | stdout           | Output file path                |
| `--list`            |                  | List available templates        |

**Examples:**

```bash
aether template --list
aether template web-app --workload-name my-site --output my-site.yaml
aether template rest-api --registry docker.io/myorg --project backend
aether template database --workload-name pg-primary
```

---

### deps

Manage workload dependencies and startup order.

```
aether deps <SUBCOMMAND>
```

| Subcommand                         | Description                        |
|------------------------------------|------------------------------------|
| `add <WORKLOAD> <DEPENDENCY>`      | Add a dependency                   |
| `remove <WORKLOAD> <DEPENDENCY>`   | Remove a dependency                |
| `show`                             | Show dependency graph              |
| `impact <WORKLOAD>`                | Show impact of stopping a workload |
| `order`                            | Show startup order                 |

**Examples:**

```bash
aether deps add web-frontend api-service
aether deps show
aether deps order
aether deps impact api-service
aether deps remove web-frontend api-service
```

---

## 🖥 UI and Utilities

### tui

Launch the interactive TUI dashboard with live workload status.

```
aether tui
```

---

### serve

Start the API server and web dashboard.

```
aether serve [--host <HOST>] [--port <PORT>]
```

| Flag      | Default       | Description    |
|-----------|---------------|----------------|
| `--host`  | `127.0.0.1`   | Server host    |
| `--port`  | `8080`         | Server port    |

**Examples:**

```bash
aether serve
aether serve --host 0.0.0.0 --port 9090
```

---

### help-all

Show detailed command reference with examples for all commands.

```
aether help-all
```

---

### completions

Generate shell completions.

```
aether completions <SHELL>
```

**Shells:** `bash`, `zsh`, `fish`, `powershell`, `elvish`

**Examples:**

```bash
aether completions bash > ~/.local/share/bash-completion/completions/aether
aether completions zsh > ~/.zsh/completions/_aether
aether completions fish > ~/.config/fish/completions/aether.fish
```

---

### config

Show or initialize configuration.

```
aether config [--show] [--init]
```

| Flag      | Description                          |
|-----------|--------------------------------------|
| `--show`  | Display current configuration        |
| `--init`  | Initialize default configuration file|

**Examples:**

```bash
aether config --show
aether config --init
```

---

### metrics

Export Prometheus-format metrics for all recorded operations.

```
aether metrics
```

---

## 🔗 Cross-References

| Document                                                             | Description                       |
|----------------------------------------------------------------------|-----------------------------------|
| [Tutorial 1: Beginner Deployment](../../tutorials/01-beginner-deployment.md)     | First deployment walkthrough     |
| [Tutorial 2: Intermediate Workflows](../../tutorials/02-intermediate-workflows.md) | Compose, migration, watch       |
| [Tutorial 3: Advanced Features](../../tutorials/03-advanced-features.md)         | Policies, secrets, drift, plugins|
| [Migration Checklist](../operations/MIGRATION_CHECKLIST.md)          | Step-by-step migration guide      |

---

> 🏷 **License:** Proprietary HyperSDK
