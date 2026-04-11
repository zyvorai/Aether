# Quick Start Guide 🚀

Deploy your first workload in 10 minutes.

---

## Prerequisites ✅

- 🖥️ Orchestr8 installed ([Installation Guide](01-Installation.md))
- 🐳 At least one runtime available (Podman recommended for local dev)
- 📝 A text editor for YAML files

---

## 1. Create a Workload Spec 📝

Create `my-app.yaml`:

```yaml
apiVersion: orchestr8/v1
kind: Workload

metadata:
  name: my-app
  owner: my-team
  project: demo

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
  allow: [container, kube]

network:
  service: true
  ports:
    - containerPort: 8080
      servicePort: 8080
      protocol: TCP
```

---

## 2. Validate ✔️

```bash
orchestr8 validate --spec my-app.yaml
```

---

## 3. Deploy 🚀

```bash
# Auto-select runtime (interactive menu)
orchestr8 run --spec my-app.yaml

# Or specify runtime explicitly
orchestr8 run --spec my-app.yaml --runtime podman

# Preview without deploying
orchestr8 run --spec my-app.yaml --dry-run
```

---

## 4. Monitor 📊

```bash
# Check status
orchestr8 status my-app

# View logs
orchestr8 logs my-app --follow

# List all workloads
orchestr8 list

# JSON output for scripting
orchestr8 list --output json
```

---

## 5. Interactive Dashboard 🖥️

```bash
orchestr8 tui
```

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate workloads |
| `/` | Search/filter |
| `Enter` | View logs |
| `r` | Refresh |
| `q` | Quit |

---

## 6. Migrate to Another Runtime 🔄

```bash
# Blue-green migration (zero downtime)
orchestr8 migrate my-app kubernetes --strategy blue-green

# Check new status
orchestr8 status my-app
```

---

## 7. Clean Up 🧹

```bash
orchestr8 stop my-app
orchestr8 delete my-app
```

---

## Next Steps

- [Beginner Tutorial](../tutorials/01-beginner-deployment.md) — Detailed walkthrough
- [Compose Guide](../features/compose.md) — Deploy multiple workloads
- [Migration Checklist](../guides/operations/MIGRATION_CHECKLIST.md) — Production migrations
- [CLI Reference](../guides/cli/CLI-Reference.md) — All commands
