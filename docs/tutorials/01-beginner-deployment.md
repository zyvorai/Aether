# Tutorial: First Deployment 🎯

**Duration:** 30 minutes | **Level:** Beginner

Learn to create, deploy, monitor, and manage workloads with Orchestr8.

---

## Prerequisites

- Orchestr8 installed ([Installation Guide](../getting-started/01-Installation.md))
- Podman available (`podman version`)
- A Dockerfile in your project (or use the sample from `orchestr8 init`)

---

## Step 1: Initialize Your Project

```bash
orchestr8 init
```

This creates:
- `~/.orchestr8/` — configuration directory
- `workload.yaml` — sample workload specification

---

## Step 2: Understand the Workload Spec

Open `workload.yaml`:

```yaml
apiVersion: orchestr8/v1
kind: Workload

metadata:
  name: hello-world      # Unique workload name
  owner: team             # Team ownership
  project: demo           # Project grouping

build:
  context: "."            # Docker build context
  dockerfile: Dockerfile  # Dockerfile path
  registry: localhost     # Image registry

requirements:
  cpu: "1"                # CPU cores
  memory: 512Mi           # Memory limit
  storage: 1Gi            # Storage limit

runtime:
  preferred: auto         # Let engine decide
  allow: [container, kube]

network:
  service: true
  ports:
    - containerPort: 8080
      servicePort: 8080
      protocol: TCP
```

---

## Step 3: Validate

```bash
orchestr8 validate
# ✅ Workload 'hello-world' is valid
```

---

## Step 4: Get a Recommendation

```bash
orchestr8 recommend
# Shows which runtime the AI engine recommends and why
```

---

## Step 5: Deploy

```bash
orchestr8 run --runtime podman
```

You'll see:
```
🚀 Deploy — Orchestr8
──────────────────────
  Building image...
  ✅ Image built: localhost/hello-world:latest
  Running on 🐳 Podman...
  ✅ Deployed 'hello-world' on Podman
```

---

## Step 6: Check Status

```bash
orchestr8 status hello-world
```

```
📊 Status for 'hello-world'
┌──────────┬─────────────────┐
│ Property │ Value           │
├──────────┼─────────────────┤
│ Runtime  │ 🐳 Podman       │
│ State    │ running         │
│ Ready    │ ● Yes           │
└──────────┴─────────────────┘
```

---

## Step 7: View Logs

```bash
orchestr8 logs hello-world
orchestr8 logs hello-world --follow   # Stream logs
```

---

## Step 8: List All Workloads

```bash
orchestr8 list
orchestr8 list --output wide    # Extra columns
orchestr8 list --output json    # Machine-readable
```

---

## Step 9: Launch the TUI Dashboard

```bash
orchestr8 tui
```

The interactive dashboard shows all workloads with:
- Real-time status
- Resource details (CPU, memory, storage)
- Health bar
- Search/filter (`/` key)

---

## Step 10: Clean Up

```bash
orchestr8 stop hello-world
orchestr8 delete hello-world
```

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Podman not found" | `sudo dnf install podman` or `sudo apt install podman` |
| "Permission denied" | Run with `sudo` or add user to `podman` group |
| "Image build failed" | Ensure Dockerfile exists in the build context |

---

## Next Steps

- [Intermediate Workflows](02-intermediate-workflows.md) — Compose, migrations, output formats
- [Advanced Features](03-advanced-features.md) — Policies, secrets, plugins
- [CLI Reference](../guides/cli/CLI-Reference.md) — All commands
