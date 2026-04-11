# 📖 Orchestr8 CLI Reference

> **Version:** See `orchestr8 --version`
> **License:** Proprietary HyperSDK

Orchestr8 is the Universal Runtime Control Plane. One spec, four runtimes:
Podman, Kubernetes, KubeVirt, and Metal3.

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
| `--verbose`       | `-v`  |                  | Enable debug-level logging                       |
| `--quiet`         | `-q`  |                  | Suppress all output except errors                |
| `--json`          |       |                  | Output results as JSON (conflicts with `--quiet`)|
| `--output <FMT>`  | `-o`  | `table`          | Output format: `table`, `json`, `yaml`, `wide`   |
| `--yes`           | `-y`  |                  | Skip confirmation prompts (CI/automation)        |
| `--dry-run`       |       |                  | Show what would happen without executing         |
| `--skip-policy`   |       |                  | Skip policy checks on deploy (use with caution)  |

---

## 📦 Workload Lifecycle

### validate

Validate a workload specification file against the schema.

```
orchestr8 [OPTIONS] validate
```

**Examples:**

```bash
orchestr8 validate
orchestr8 --spec my-app.yaml validate
orchestr8 --output json validate
```

---

### build

Build a container image from the workload spec. The runtime is selected
automatically by the decision engine.

```
orchestr8 [OPTIONS] build
```

**Examples:**

```bash
orchestr8 build
orchestr8 --spec api.yaml build
```

---

### run

Deploy a workload instance. Builds the image, starts the instance, and persists
state. Shows an interactive runtime selector unless `--runtime` is provided.

```
orchestr8 [OPTIONS] run [--runtime <RUNTIME>]
```

| Flag               | Short | Description                                        |
|--------------------|-------|----------------------------------------------------|
| `--runtime <NAME>` | `-r`  | Override runtime: `podman`, `kube`, `kubevirt`, `metal` |

**Examples:**

```bash
orchestr8 run
orchestr8 run --runtime podman
orchestr8 --dry-run run
orchestr8 --skip-policy run --runtime kube
```

---

### stop

Stop a running workload instance. The state is preserved for restart.

```
orchestr8 [OPTIONS] stop <NAME>
```

**Examples:**

```bash
orchestr8 stop hello-web
orchestr8 --dry-run stop hello-web
```

---

### status

Get the current status of a workload instance.

```
orchestr8 [OPTIONS] status <NAME>
```

**Examples:**

```bash
orchestr8 status hello-web
orchestr8 --output json status hello-web
orchestr8 --output yaml status hello-web
```

---

### logs

View logs from a workload instance.

```
orchestr8 [OPTIONS] logs <NAME> [--follow]
```

| Flag       | Short | Description                |
|------------|-------|----------------------------|
| `--follow` | `-f`  | Follow log output in real time |

**Examples:**

```bash
orchestr8 logs hello-web
orchestr8 logs hello-web --follow
```

---

### delete

Delete a workload instance and remove it from state.

```
orchestr8 [OPTIONS] delete <NAME>
```

**Examples:**

```bash
orchestr8 delete hello-web
orchestr8 --yes delete hello-web
orchestr8 --dry-run delete hello-web
```

---

### list

List all tracked workload instances.

```
orchestr8 [OPTIONS] list
```

**Examples:**

```bash
orchestr8 list
orchestr8 --output wide list
orchestr8 --json list
```

---

## 🔀 Migration

### migrate

Migrate a workload from its current runtime to a different one.

```
orchestr8 [OPTIONS] migrate <NAME> <TARGET> [--strategy <STRATEGY>] [--no-validation] [--no-rollback]
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
orchestr8 migrate hello-web kube
orchestr8 migrate hello-web kube --strategy rolling
orchestr8 migrate hello-web podman --strategy immediate --no-rollback
orchestr8 --dry-run migrate hello-web kubevirt --strategy blue-green
```

---

### rollback

Rollback a workload to its latest pre-deploy snapshot.

```
orchestr8 [OPTIONS] rollback <NAME>
```

**Examples:**

```bash
orchestr8 rollback hello-web
```

---

## 🛠 Developer Workflow

### exec

Execute a command inside a running workload container.

```
orchestr8 [OPTIONS] exec <NAME> [COMMAND] [--interactive] [--timeout <SECS>]
```

| Flag              | Short | Default   | Description                      |
|-------------------|-------|-----------|----------------------------------|
| `COMMAND`         |       | `/bin/sh` | Command to execute               |
| `--interactive`   | `-i`  |           | Pass stdin to the container      |
| `--timeout`       | `-t`  | `0`       | Timeout in seconds (0 = none)    |

**Examples:**

```bash
orchestr8 exec hello-web
orchestr8 exec hello-web "ls -la /app" --timeout 30
orchestr8 exec hello-web "/bin/bash" --interactive
```

---

### port-forward

Forward local ports to a running workload.

```
orchestr8 [OPTIONS] port-forward <NAME> <PORTS> [--timeout <SECS>]
```

| Flag          | Short | Default | Description                      |
|---------------|-------|---------|----------------------------------|
| `PORTS`       |       |         | Port mapping `local:remote`      |
| `--timeout`   | `-t`  | `0`     | Timeout in seconds (0 = none)    |

**Examples:**

```bash
orchestr8 port-forward hello-web 8080:80
orchestr8 port-forward hello-web 3000:3000 --timeout 300
```

---

### watch

Watch the spec file and auto-redeploy on changes.

```
orchestr8 [OPTIONS] watch [--runtime <RUNTIME>]
```

**Examples:**

```bash
orchestr8 watch
orchestr8 --spec api.yaml watch --runtime podman
```

---

### compare

Compare a workload across all four runtimes (cost, capabilities, limitations).

```
orchestr8 [OPTIONS] compare
```

**Examples:**

```bash
orchestr8 compare
orchestr8 --output json compare
```

---

### init

Run the first-time setup wizard. Detects runtimes, creates config, ensures directories.

```
orchestr8 init
```

---

### health

View health history and uptime statistics for a workload.

```
orchestr8 [OPTIONS] health <NAME> [--last <N>] [--summary]
```

| Flag        | Short | Default | Description                       |
|-------------|-------|---------|-----------------------------------|
| `--last`    | `-l`  | `20`    | Show last N health records        |
| `--summary` |       |         | Show summary statistics only      |

**Examples:**

```bash
orchestr8 health hello-web
orchestr8 health hello-web --last 50
orchestr8 health hello-web --summary
```

---

### diff

Three-way diff: spec file vs stored state vs live runtime.

```
orchestr8 [OPTIONS] diff <NAME>
```

**Examples:**

```bash
orchestr8 diff hello-web
orchestr8 --output json diff hello-web
```

---

## 🗂 Compose

### compose validate

Validate a compose file for structural correctness, dependency cycles, and
missing references.

```
orchestr8 compose validate [FILE]
```

**Examples:**

```bash
orchestr8 compose validate
orchestr8 compose validate ./deploy/orchestr8-compose.yaml
```

---

### compose up

Deploy all workloads defined in a compose file, in dependency order.

```
orchestr8 compose up [FILE] [--runtime <RUNTIME>] [--dry-run]
```

| Flag          | Short | Description                              |
|---------------|-------|------------------------------------------|
| `--runtime`   | `-r`  | Override runtime for all workloads       |
| `--dry-run`   |       | Show deployment plan without executing   |

**Examples:**

```bash
orchestr8 compose up
orchestr8 compose up --runtime podman
orchestr8 compose up --dry-run
orchestr8 compose up ./deploy/orchestr8-compose.yaml
```

---

### compose down

Stop and remove all workloads from a compose file, in reverse dependency order.

```
orchestr8 compose down [FILE]
```

**Examples:**

```bash
orchestr8 compose down
orchestr8 compose down ./deploy/orchestr8-compose.yaml
```

---

## 🧩 Plugins

### plugin list

List all registered plugins.

```
orchestr8 plugin list
```

---

### plugin discover

Scan `~/.orchestr8/plugins/` for `*.json` manifest files and register them.

```
orchestr8 plugin discover
```

---

### plugin register

Register a plugin from a manifest file.

```
orchestr8 plugin register <MANIFEST>
```

**Examples:**

```bash
orchestr8 plugin register ./wasm-runtime.json
```

---

### plugin remove

Unregister a plugin by name.

```
orchestr8 plugin remove <NAME>
```

**Examples:**

```bash
orchestr8 plugin remove wasm-runtime
```

---

## 🔧 Operations

### backup

Create a backup of the current workload state.

```
orchestr8 [OPTIONS] backup [--name <NAME>] [--description <DESC>]
```

| Flag              | Short | Description                                    |
|-------------------|-------|------------------------------------------------|
| `--name`          | `-n`  | Backup name (auto-generated if not provided)   |
| `--description`   | `-d`  | Human-readable description                     |

**Examples:**

```bash
orchestr8 backup
orchestr8 backup --name pre-migration --description "Before kube migration"
```

---

### restore

Restore workload state from a backup file.

```
orchestr8 [OPTIONS] restore <BACKUP> [--merge]
```

| Flag      | Short | Description                                     |
|-----------|-------|-------------------------------------------------|
| `--merge` | `-m`  | Merge with existing state instead of replacing  |

**Examples:**

```bash
orchestr8 restore ~/.orchestr8/backups/backup-20260411-103000.json
orchestr8 restore backup.json --merge
```

---

### list-backups

List all available backup files.

```
orchestr8 list-backups
```

---

### deploy

Deploy all workload specs from a directory.

```
orchestr8 [OPTIONS] deploy <DIR> [--runtime <RUNTIME>] [--fail-fast] [--dry-run]
```

| Flag           | Short | Description                              |
|----------------|-------|------------------------------------------|
| `--runtime`    | `-r`  | Override runtime for all workloads       |
| `--fail-fast`  |       | Stop on first failure                    |
| `--dry-run`    |       | Show deployment plan without executing   |

**Examples:**

```bash
orchestr8 deploy ./specs/
orchestr8 deploy ./specs/ --runtime podman --fail-fast
orchestr8 deploy ./specs/ --dry-run
```

---

### drift

Detect configuration drift between the desired spec and live state.

```
orchestr8 [OPTIONS] drift <NAME> [--reconcile]
```

| Flag           | Description                                     |
|----------------|-------------------------------------------------|
| `--reconcile`  | Automatically reconcile detected drift          |

**Examples:**

```bash
orchestr8 drift hello-web
orchestr8 drift hello-web --reconcile
```

---

## 🏛 Governance

### policy-check

Evaluate a workload spec against deployment policies.

```
orchestr8 [OPTIONS] policy-check [--policy <POLICY>]
```

| Flag        | Short | Default      | Description                                     |
|-------------|-------|--------------|-------------------------------------------------|
| `--policy`  | `-p`  | `production` | Policy set: `production`, `development`, or file path |

**Examples:**

```bash
orchestr8 policy-check
orchestr8 policy-check --policy development
orchestr8 policy-check --policy ./custom-policies.yaml
```

---

### secrets

Manage encrypted secrets (AES-256-GCM when `ORCHESTR8_SECRET_KEY` is set).

```
orchestr8 secrets <SUBCOMMAND>
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
orchestr8 secrets create db-creds --namespace production
orchestr8 secrets set db-creds password "s3cret"
orchestr8 secrets get db-creds password
orchestr8 secrets list
orchestr8 secrets audit
```

---

### sla

SLA compliance monitoring.

```
orchestr8 sla <SUBCOMMAND>
```

| Subcommand                                                                   | Description              |
|------------------------------------------------------------------------------|--------------------------|
| `add <WORKLOAD> [--tier <TIER>]`                                             | Add an SLA target        |
| `check <WORKLOAD> --uptime <PCT> [--latency <MS>] [--error-rate <PCT>] [--restarts <N>]` | Check compliance |
| `list`                                                                       | List all SLA targets     |

**Tiers:** `standard`, `high-availability`, `best-effort`

**Examples:**

```bash
orchestr8 sla add hello-web --tier high-availability
orchestr8 sla check hello-web --uptime 99.9 --latency 50 --error-rate 0.01
orchestr8 sla list
```

---

### audit

View the audit trail of workload operations.

```
orchestr8 [OPTIONS] audit [--last <N>] [--workload <NAME>] [--summary]
```

| Flag           | Short | Default | Description                       |
|----------------|-------|---------|-----------------------------------|
| `--last`       | `-l`  | `20`    | Show last N events                |
| `--workload`   | `-w`  |         | Filter by workload name           |
| `--summary`    |       |         | Show summary only                 |

**Examples:**

```bash
orchestr8 audit
orchestr8 audit --last 50 --workload hello-web
orchestr8 audit --summary
```

---

### events

View and manage the event stream.

```
orchestr8 [OPTIONS] events [--last <N>] [--severity <LEVEL>] [--summary]
```

| Flag          | Default | Description                                            |
|---------------|---------|--------------------------------------------------------|
| `--last`      | `20`    | Show last N events                                     |
| `--severity`  |         | Filter: `info`, `warning`, `error`, `critical`        |
| `--summary`   |         | Show summary only                                      |

**Examples:**

```bash
orchestr8 events
orchestr8 events --last 100 --severity error
orchestr8 events --summary
```

---

## 🤖 Advanced / AI

### cost

Estimate workload costs across cloud providers.

```
orchestr8 [OPTIONS] cost [--provider <PROVIDER>]
```

| Flag          | Short | Default | Description                                       |
|---------------|-------|---------|---------------------------------------------------|
| `--provider`  | `-c`  | `all`   | `aws`, `azure`, `gcp`, `digitalocean`, `linode`, `all` |

**Examples:**

```bash
orchestr8 cost
orchestr8 cost --provider aws
orchestr8 --output json cost --provider all
```

---

### recommend

AI-powered runtime recommendation with scoring.

```
orchestr8 [OPTIONS] recommend [--runtime <RUNTIME>]
```

**Examples:**

```bash
orchestr8 recommend
orchestr8 recommend --runtime kube
```

---

### profile

Profile a workload and show optimization recommendations.

```
orchestr8 [OPTIONS] profile [--name <NAME>]
```

**Examples:**

```bash
orchestr8 profile
orchestr8 profile --name hello-web
```

---

### analyze-logs

Analyze workload logs for anomalies and patterns.

```
orchestr8 [OPTIONS] analyze-logs <NAME>
```

**Examples:**

```bash
orchestr8 analyze-logs hello-web
```

---

### migration-advice

Get AI-powered migration advice for a workload.

```
orchestr8 [OPTIONS] migration-advice <NAME> <TARGET>
```

**Examples:**

```bash
orchestr8 migration-advice hello-web kube
orchestr8 migration-advice api-service kubevirt
```

---

### scaling-advice

Show predictive scaling recommendations based on historical data.

```
orchestr8 [OPTIONS] scaling-advice
```

**Examples:**

```bash
orchestr8 scaling-advice
orchestr8 --output json scaling-advice
```

---

## 🎛 Orchestration

### orchestrate

Health-aware orchestration with circuit breakers and rolling updates.

```
orchestr8 orchestrate <SUBCOMMAND>
```

| Subcommand                                       | Description                                   |
|--------------------------------------------------|-----------------------------------------------|
| `register <NAME> [--runtime <RT>]`               | Register a workload for health monitoring     |
| `status`                                         | Show health status of all workloads           |
| `summary`                                        | Show health summary                           |
| `rolling-update <NAME> [--replicas <N>]`         | Simulate a rolling update                     |
| `reset-circuit <NAME>`                           | Reset circuit breaker for a workload          |
| `health-check`                                   | Run a single round of health checks           |
| `watch [--interval <SECS>]`                      | Continuously monitor health (default: 30s)    |

**Examples:**

```bash
orchestr8 orchestrate register api --runtime kubernetes
orchestr8 orchestrate status
orchestr8 orchestrate watch --interval 60
orchestr8 orchestrate rolling-update api --replicas 5
orchestr8 orchestrate reset-circuit api
orchestr8 orchestrate health-check
```

---

### schedule

Workload scheduling and placement optimization.

```
orchestr8 schedule <SUBCOMMAND>
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
orchestr8 schedule place api --cpu 4 --memory 8192 --strategy cost
orchestr8 schedule utilization
orchestr8 schedule optimize
orchestr8 schedule placements
```

---

### affinity

Runtime affinity learning and recommendations.

```
orchestr8 affinity <SUBCOMMAND>
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
orchestr8 affinity recommend web-service
orchestr8 affinity matrix
orchestr8 affinity stats
```

---

### webhook

Manage webhook notification channels with retry queue.

```
orchestr8 webhook <SUBCOMMAND>
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
orchestr8 webhook add slack "https://hooks.slack.com/..." --severity warning
orchestr8 webhook test slack
orchestr8 webhook queue
orchestr8 webhook flush
orchestr8 webhook list
orchestr8 webhook remove slack
```

---

### env

Manage deployment environments.

```
orchestr8 env <SUBCOMMAND>
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
orchestr8 env create staging --tier staging
orchestr8 env list
orchestr8 env promote hello-web staging production
orchestr8 env parity staging production
```

---

### template

Generate a workload spec from a built-in template.

```
orchestr8 template <NAME> [--workload-name <WN>] [--owner <O>] [--project <P>] [--registry <R>] [--output <FILE>] [--list]
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
orchestr8 template --list
orchestr8 template web-app --workload-name my-site --output my-site.yaml
orchestr8 template rest-api --registry docker.io/myorg --project backend
orchestr8 template database --workload-name pg-primary
```

---

### deps

Manage workload dependencies and startup order.

```
orchestr8 deps <SUBCOMMAND>
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
orchestr8 deps add web-frontend api-service
orchestr8 deps show
orchestr8 deps order
orchestr8 deps impact api-service
orchestr8 deps remove web-frontend api-service
```

---

## 🖥 UI and Utilities

### tui

Launch the interactive TUI dashboard with live workload status.

```
orchestr8 tui
```

---

### serve

Start the API server and web dashboard.

```
orchestr8 serve [--host <HOST>] [--port <PORT>]
```

| Flag      | Default       | Description    |
|-----------|---------------|----------------|
| `--host`  | `127.0.0.1`   | Server host    |
| `--port`  | `8080`         | Server port    |

**Examples:**

```bash
orchestr8 serve
orchestr8 serve --host 0.0.0.0 --port 9090
```

---

### help-all

Show detailed command reference with examples for all commands.

```
orchestr8 help-all
```

---

### completions

Generate shell completions.

```
orchestr8 completions <SHELL>
```

**Shells:** `bash`, `zsh`, `fish`, `powershell`, `elvish`

**Examples:**

```bash
orchestr8 completions bash > ~/.local/share/bash-completion/completions/orchestr8
orchestr8 completions zsh > ~/.zsh/completions/_orchestr8
orchestr8 completions fish > ~/.config/fish/completions/orchestr8.fish
```

---

### config

Show or initialize configuration.

```
orchestr8 config [--show] [--init]
```

| Flag      | Description                          |
|-----------|--------------------------------------|
| `--show`  | Display current configuration        |
| `--init`  | Initialize default configuration file|

**Examples:**

```bash
orchestr8 config --show
orchestr8 config --init
```

---

### metrics

Export Prometheus-format metrics for all recorded operations.

```
orchestr8 metrics
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
