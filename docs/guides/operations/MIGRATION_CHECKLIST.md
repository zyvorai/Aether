# Migration Checklist ✅

Production-ready checklist for migrating workloads between runtimes.

---

## Pre-Migration

- [ ] **Source workload is running:** `orchestr8 status <name>`
- [ ] **Target runtime is available:** `kubectl cluster-info` / `podman version` / `virtctl version`
- [ ] **Spec is valid:** `orchestr8 validate --spec <spec>`
- [ ] **Policy check passes:** `orchestr8 policy-check --spec <spec> --policy production`
- [ ] **Backup created:** `orchestr8 backup <name> --description "pre-migration"`
- [ ] **Compare runtimes:** `orchestr8 compare`
- [ ] **Get migration advice:** `orchestr8 migration-advice <name> <target>`

---

## Strategy Selection

| Criteria | Immediate | Blue-Green | Rolling |
|----------|-----------|------------|---------|
| **Downtime tolerance** | Seconds OK | Zero required | Zero required |
| **Resource overhead** | 1x | 2x temporarily | 1.5x gradually |
| **Rollback speed** | Redeploy | Instant (switch back) | Gradual |
| **Validation** | Post-deploy | Before switch | Continuous |
| **Best for** | Dev/test | Production | Critical services |

---

## Execution

### 1. Preview (Dry-Run)

```bash
orchestr8 migrate <name> <target> --strategy blue-green --dry-run
```

### 2. Execute

```bash
orchestr8 migrate <name> <target> --strategy blue-green
```

### 3. Monitor

```bash
orchestr8 status <name>
orchestr8 health <name> --summary
orchestr8 tui
```

---

## Post-Migration

- [ ] **Status is healthy:** `orchestr8 status <name>` shows `running` + `ready`
- [ ] **Health uptime:** `orchestr8 health <name> --summary`
- [ ] **Logs are clean:** `orchestr8 logs <name>`
- [ ] **No drift detected:** `orchestr8 drift <name>`
- [ ] **Application responds:** test your endpoints

---

## Rollback

If something goes wrong:

```bash
# Rollback to previous state
orchestr8 rollback <name>

# Or migrate back
orchestr8 migrate <name> <original-runtime> --strategy immediate
```

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Source and target runtimes are the same" | Use `rollback` instead of `migrate` |
| "Target instance not ready" | Check target runtime health, increase validation delay |
| "Rollback performed" | Check logs for why target failed validation |
| "Connection refused" | Verify `kubectl cluster-info` or `podman version` |
