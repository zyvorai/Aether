# 🚀 Tutorial 1: Your First Deployment with Orchestr8

> **Estimated time:** 30--45 minutes
> **Level:** Beginner
> **License:** Proprietary HyperSDK

---

## 📑 Table of Contents

- [System Requirements](#-system-requirements)
- [Installation Walkthrough](#-installation-walkthrough)
- [Create a Workload Spec](#-create-a-workload-spec)
- [Validate the Spec](#-validate-the-spec)
- [Deploy to Podman](#-deploy-to-podman)
- [Check Status and View Logs](#-check-status-and-view-logs)
- [Stop and Delete](#-stop-and-delete)
- [Understanding the TUI Dashboard](#-understanding-the-tui-dashboard)
- [Troubleshooting Common Issues](#-troubleshooting-common-issues)
- [Next Steps](#-next-steps)

---

## 🖥 System Requirements

| Component        | Minimum                     | Recommended                 |
|------------------|-----------------------------|-----------------------------|
| **OS**           | Linux (x86_64 / aarch64)   | Fedora 40+, Ubuntu 24.04+  |
| **Rust**         | 1.75+                       | latest stable               |
| **Podman**       | 4.0+                        | 5.0+                        |
| **kubectl**      | 1.28+ *(optional)*          | 1.30+                       |
| **Disk**         | 500 MB                      | 2 GB                        |
| **RAM**          | 2 GB                        | 8 GB                        |

> 💡 **Tip:** Podman is the only runtime required for this beginner tutorial.
> Kubernetes, KubeVirt, and Metal3 are covered in later guides.

---

## 📦 Installation Walkthrough

### 1. Install Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. Clone and build Orchestr8

```bash
git clone https://github.com/example/orchestr8.git
cd orchestr8
cargo build --release
```

### 3. Add to your PATH

```bash
sudo cp target/release/orchestr8 /usr/local/bin/
```

### 4. Verify the installation

```bash
orchestr8 --version
```

### 5. Run the first-time setup wizard

```bash
orchestr8 init
```

The wizard detects available runtimes, creates the default config at
`~/.orchestr8/config.yaml`, and ensures the state directory exists.

### 6. Install shell completions *(optional)*

```bash
# Bash
orchestr8 completions bash > ~/.local/share/bash-completion/completions/orchestr8

# Zsh
orchestr8 completions zsh > ~/.zsh/completions/_orchestr8

# Fish
orchestr8 completions fish > ~/.config/fish/completions/orchestr8.fish
```

---

## 📝 Create a Workload Spec

Create a file named `workload.yaml` in your project directory:

```yaml
# workload.yaml — Orchestr8 Universal Workload Specification
apiVersion: orchestr8/v1
kind: Workload

metadata:
  name: hello-web
  owner: my-team
  project: getting-started
  labels:
    app: hello-web
    environment: dev
  annotations:
    description: "A simple web application for the tutorial"

build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/my-org
  buildArgs:
    NODE_ENV: production

requirements:
  cpu: "2"
  memory: 4Gi
  storage: 20Gi

runtime:
  preferred: container          # auto | container | kube | kubevirt | metal
  allow:
    - container
    - kube

network:
  service: false
  serviceType: ClusterIP
  ports:
    - containerPort: 8080
      servicePort: 80
      protocol: TCP

persistence:
  enabled: false
  size: 10Gi
  accessMode: ReadWriteOnce

health:
  liveness:
    path: /healthz
    port: 8080
    initialDelaySeconds: 10
    periodSeconds: 30
  readiness:
    path: /ready
    port: 8080
    initialDelaySeconds: 5
    periodSeconds: 10
```

### Spec anatomy at a glance

| Section          | Purpose                                                    |
|------------------|------------------------------------------------------------|
| `metadata`       | Name, owner, project, labels, annotations                  |
| `build`          | Dockerfile context, registry, build arguments               |
| `requirements`   | CPU, memory, storage, optional GPU                          |
| `runtime`        | Preferred runtime and allow-list                            |
| `network`        | Service exposure, port mappings, service type               |
| `persistence`    | Persistent volume claims, access modes, storage classes     |
| `health`         | Liveness and readiness probes                               |

---

## ✅ Validate the Spec

Before deploying, always validate:

```bash
orchestr8 validate
```

By default, Orchestr8 looks for `workload.yaml` in the current directory.
To use a different file:

```bash
orchestr8 --spec my-workload.yaml validate
```

A successful validation prints a property table:

```
✔ Workload specification is valid
✔ Workload 'hello-web' is valid

  Name               hello-web
  Owner              my-team
  Project            getting-started
  CPU                2
  Memory             4Gi
  Storage            20Gi
  Preferred Runtime  Container
```

---

## 🐳 Deploy to Podman

### Deploy the workload

```bash
orchestr8 run
```

Orchestr8 will:

1. Evaluate the runtime decision engine (rule-based + AI scoring)
2. Show an interactive runtime selector (unless `--runtime` is passed)
3. Build the container image via Podman
4. Start the workload instance
5. Persist the state to `~/.orchestr8/state.json`

#### Override the runtime explicitly

```bash
orchestr8 run --runtime podman
```

#### Dry-run mode (preview without executing)

```bash
orchestr8 --dry-run run
```

This prints what *would* happen without creating any resources.

---

## 📊 Check Status and View Logs

### Check status

```bash
orchestr8 status hello-web
```

Sample output:

```
  Workload   hello-web
  Runtime    podman
  State      running
  Ready      true
  Restarts   0
```

### View logs

```bash
orchestr8 logs hello-web
```

#### Follow logs in real-time

```bash
orchestr8 logs hello-web --follow
```

### List all running workloads

```bash
orchestr8 list
```

```
┌─────────────┬─────────┬─────────┬──────────────────────┐
│ Name        │ Runtime │ State   │ Created              │
├─────────────┼─────────┼─────────┼──────────────────────┤
│ hello-web   │ podman  │ running │ 2026-04-11T10:30:00Z │
└─────────────┴─────────┴─────────┴──────────────────────┘
```

### Output formats

```bash
# JSON (machine-readable, great for scripting)
orchestr8 --output json list

# YAML
orchestr8 --output yaml status hello-web

# Wide table (extra columns)
orchestr8 --output wide list
```

---

## 🛑 Stop and Delete

### Stop a workload (preserves state)

```bash
orchestr8 stop hello-web
```

### Delete a workload (removes state and resources)

```bash
orchestr8 delete hello-web
```

> ⚠️ **Warning:** `delete` is irreversible. Use `--dry-run` to preview:
>
> ```bash
> orchestr8 --dry-run delete hello-web
> ```

### Automatic confirmation for CI/CD

```bash
orchestr8 --yes delete hello-web
```

---

## 🖥 Understanding the TUI Dashboard

Launch the interactive terminal dashboard:

```bash
orchestr8 tui
```

The TUI provides a live view of all deployed workloads with:

| Panel               | Description                                         |
|----------------------|-----------------------------------------------------|
| **Workload List**    | All tracked workloads, runtime, state               |
| **Status Detail**    | Selected workload details, health, restart count    |
| **Logs**             | Live log stream from the selected workload          |
| **Resource Usage**   | CPU/Memory utilization gauges                       |

### Keyboard shortcuts

| Key          | Action                  |
|--------------|-------------------------|
| `j` / `k`   | Navigate up/down        |
| `Enter`      | Select workload         |
| `l`          | Toggle log panel        |
| `q`          | Quit dashboard          |
| `r`          | Refresh data            |
| `/`          | Search / filter         |

---

## 🔧 Troubleshooting Common Issues

### "Workload not found" when running status or logs

```
Error: Workload 'hello-web' not found.
Hint: Run `orchestr8 list` to see deployed workloads.
```

**Cause:** The workload has not been deployed yet, or was deleted.
**Fix:** Run `orchestr8 list` and verify the name. Redeploy with `orchestr8 run`.

---

### "No suitable runtime found"

```
Error: No suitable runtime found for workload
```

**Cause:** The `runtime.allow` list in your spec is empty.
**Fix:** Add at least one runtime to the allow list:

```yaml
runtime:
  preferred: auto
  allow:
    - container
```

---

### Podman not detected

```
Error: podman binary not found in PATH
```

**Fix:** Install Podman:

```bash
# Fedora
sudo dnf install podman

# Ubuntu
sudo apt install podman
```

---

### Validation failures

Run `orchestr8 validate` and read the error message carefully. Common issues:

| Error                            | Fix                                          |
|----------------------------------|----------------------------------------------|
| Missing `metadata.name`         | Add a `name` field under `metadata`          |
| Invalid CPU format              | Use `"2"` or `"2000m"` (quotes required)     |
| Invalid memory format           | Use `4Gi`, `4096Mi`, or `512Ki`              |
| Unknown runtime in allow list   | Valid values: `container`, `kube`, `kubevirt`, `metal` |

---

### Policy check blocks deployment

```
Error: Policy violation: CPU exceeds maximum allowed (16 cores)
```

**Fix:** Either reduce resources in your spec, or skip policy checks:

```bash
orchestr8 --skip-policy run
```

> ⚠️ Use `--skip-policy` only in development. Production policies exist for a reason.

---

## 🎯 Next Steps

Congratulations -- you have deployed your first workload with Orchestr8! Here is where to go next:

| Tutorial                                                                 | Topics                                          |
|--------------------------------------------------------------------------|------------------------------------------------|
| [02 - Intermediate Workflows](./02-intermediate-workflows.md)            | Compose files, migration, output formats        |
| [03 - Advanced Features](./03-advanced-features.md)                      | Policies, secrets, drift detection, plugins     |
| [CLI Reference](../guides/cli/CLI-Reference.md)                         | Complete command reference                      |
| [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md)       | Pre/post migration procedures                   |

### Quick wins to try now

```bash
# Compare your workload across all runtimes
orchestr8 compare

# Get AI-powered runtime recommendations
orchestr8 recommend

# Estimate deployment costs across cloud providers
orchestr8 cost

# Generate a workload from a template
orchestr8 template web-app --workload-name my-app --output my-app.yaml

# Run a policy check
orchestr8 policy-check
```

---

> 📚 **Full documentation:** [CLI Reference](../guides/cli/CLI-Reference.md)
> 🏷 **License:** Proprietary HyperSDK
