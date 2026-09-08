# Stateful Portability

> Honest guide to volumes, databases, and persistence across runtime migrations.

---

## What Aether does

- **Deploy** stateful specs with PVC auto-mount (Kubernetes adapter)
- **Snapshot** workload state before migrate (best-effort via backup module)
- **Re-create** containers/VMs on target runtime with same spec

## What Aether does not do

- **Copy disk data** between Podman volumes, PVCs, and bare-metal disks
- **Replicate databases** automatically
- **Preserve IP/DNS** endpoints

Migration moves the **workload definition and process**, not the bytes on disk.

---

## Limits by concern

| Concern | Podman | Kubernetes | KubeVirt |
|---------|--------|------------|----------|
| Local volume | bind mount | PVC | VM disk |
| Snapshot | Manual | CSI snapshot* | VM snapshot* |
| Cross-runtime copy | Not automatic | Not automatic | Not automatic |

\* Via platform tooling + [BACKUP.md](../../BACKUP.md), not migration engine.

---

## Database migration playbook

### 1. Dump and restore

```bash
# Source still running
pg_dump -h source -U app db > backup.sql
aether migrate my-db kubernetes --strategy blue-green
# Target up — restore before cutover
psql -h target -U app db < backup.sql
```

### 2. Dual-write / read replica

For zero data loss: replicate until caught up, brief read-only window, cutover DNS.

### 3. Object storage

Prefer S3-compatible storage for blobs; migration only moves the app tier.

---

## Kubernetes PVC notes

See `examples/workload-full-featured.yaml` for persistence block. After migrate **to** K8s, ensure StorageClass exists. After migrate **from** K8s, export data before deleting PVC.

---

## Recommendations

| Workload type | Strategy |
|---------------|----------|
| Stateless app | Blue-green migrate freely |
| Cache (Redis) | Accept cold cache or replicate |
| SQL database | Dump/restore or logical replication |
| File uploads on disk | Move to object storage first |

---

## Related

- [Migration Internals](MIGRATION-INTERNALS.md)
- [BACKUP.md](../../BACKUP.md)
