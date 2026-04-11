# Orchestr8 Quick Reference Card

---

## Essential Commands

```bash
orchestr8 init                               # First-time setup
orchestr8 validate                           # Validate spec
orchestr8 run [--runtime podman|kube|...]    # Deploy workload
orchestr8 status <name>                      # Check status
orchestr8 logs <name> [--follow]             # View logs
orchestr8 list [--output wide|json|yaml]     # List workloads
orchestr8 migrate <name> <target> [--strategy blue-green]
orchestr8 tui                                # Interactive dashboard
```

## Runtime Icons

| Icon | Runtime | CLI Name |
|------|---------|----------|
| 🐳 | Podman | `podman` |
| ☸️ | Kubernetes | `kubernetes`, `kube`, `k8s` |
| 🖥️ | KubeVirt | `kubevirt`, `vm` |
| 🖧 | Metal3 | `metal3`, `metal`, `bare-metal` |

## Migration Strategies

| Strategy | Downtime | Use Case |
|----------|----------|----------|
| `immediate` | Brief | Dev/test |
| `blue-green` | Zero | Production |
| `rolling` | Zero | Critical services |

## TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate |
| `/` | Search/filter |
| `Esc` | Clear filter |
| `Enter` | View logs |
| `g/G` | First/last |
| `r` | Refresh |
| `q` | Quit |

## Output Formats

```bash
--output table     # Default
--output json      # Machine-readable
--output yaml      # YAML format
--output wide      # Extra columns
--dry-run          # Preview only
```

## Config File

`~/.orchestr8/config.yaml`

```yaml
policy:
  enforceOnDeploy: true
  policySet: production
reconciliation:
  healthIntervalSecs: 30
  driftIntervalSecs: 300
  slaIntervalSecs: 60
  autoReconcile: false
webhook:
  enabled: true
  maxRetries: 3
```

## Key Environment Variables

| Variable | Description |
|----------|-------------|
| `ORCHESTR8_SECRET_KEY` | Encryption key for AES-256-GCM secrets |
| `KUBECONFIG` | Kubernetes config path |

## State Files

| File | Purpose |
|------|---------|
| `~/.orchestr8/state.json` | Workload state (file-locked) |
| `~/.orchestr8/config.yaml` | Configuration |
| `~/.orchestr8/health.json` | Health history |
| `~/.orchestr8/secrets.json` | Encrypted secrets |
| `~/.orchestr8/plugins.json` | Plugin registry |
| `~/.orchestr8/webhook_queue.json` | Pending webhooks |
