<!-- Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved. -->
# Atlas storage integration

[Atlas](https://github.com/ssahani/atlas) is the Zyvor **storage control plane**. When
enabled, Aether provisions persistent storage for a workload through Atlas — which maps
an intent *policy* to a backend (Ceph RBD/CephFS/RGW, NFS, ZFS) and owns inventory,
quotas, snapshots, and backups — instead of creating a native Kubernetes PVC itself.

## Enabling

Set on the Aether CLI or `aether serve` gateway environment:

| Variable | Meaning |
|----------|---------|
| `AETHER_ATLAS_URL` | Atlas gateway base URL, e.g. `http://atlas:5110`. **Unset ⇒ integration off** (native storage). |
| `AETHER_ATLAS_TOKEN` | Optional `Authorization: Bearer` service-account JWT (a `product.service.aether` token → operator). |
| `AETHER_ATLAS_TENANT` | Tenant for provisioned volumes (default `default`). |

## Opting a workload in

A workload uses Atlas when its `persistence.storageClass` is `atlas/<policy>`:

```yaml
persistence:
  enabled: true
  size: 20Gi
  accessMode: ReadWriteOnce
  storageClass: atlas/database   # → Atlas "database" policy
```

Built-in policies: `production`, `database`, `development`, `shared` (RWX/CephFS), `ai`.

**Policy precedence** (`Workload::atlas_policy()`): explicit `atlas/<policy>` suffix ›
`intent.storage.tier` › access-mode default (`ReadWriteMany` → `shared`, else `production`).
Use a bare `atlas/` (or `atlas`) storage class to let intent/access-mode pick the policy:

```yaml
persistence: { enabled: true, size: 20Gi, storageClass: "atlas/" }
intent: { goal: balanced, storage: { tier: database } }
```

## What happens on deploy / delete

- **Deploy** (`aether run`): before the workload is applied, Aether calls Atlas
  `POST /api/atlas/v1/volumes` (with `create_pvc: true`) and waits for the job to
  finish. Atlas creates the PVC — named exactly `{workload}-pvc` — which the pod then
  mounts. The Atlas volume id(s) are recorded in workload state.
- **Delete** (`aether delete`): Aether releases the Atlas volume(s) first (Atlas owns
  the PVC lifecycle); the adapter's own PVC delete is then a tolerant no-op.
- **Update** (`aether update`): the volume is preserved (the workload keeps its
  `atlas_volume_ids`).

## Workload-kind support

| Kind | Support |
|------|---------|
| Deployment / Job | ✅ Atlas API creates a single tracked volume + PVC `{name}-pvc`. |
| StatefulSet | ✅ Atlas API creates one tracked volume per replica, named `{name}-storage-{name}-{ordinal}`, which the StatefulSet **adopts**. The `volumeClaimTemplate` is rewritten to the resolved concrete class, so **scale-up** dynamically provisions further PVCs on the same Atlas-backed class. |
| KubeVirt | ✅ the DataVolume's `storageClassName` is set to the Atlas-resolved class, so CDI provisions the VM disk on Atlas-backed storage (Ceph RBD/CephFS). The disk is CDI-owned (imports the OS image), so it is **not** a separately tracked Atlas volume. |
| Metal3 / Podman | Not routed through Atlas (native/no persistent storage). |

The concrete StorageClass behind a policy is resolved at deploy time from Atlas's
policy catalog (`GET /api/atlas/v1/policies`) — Aether does not hardcode class names.

## Inspecting & snapshots

```bash
aether storage status                 # endpoint, tenant, volume count, total size
aether storage list                   # volumes owned by this tenant
aether storage snapshot <name> <snap> # snapshot a workload's Atlas volume(s)
aether storage clone   <snapshot-id> <new-name>
aether storage restore <snapshot-id> <new-name>
```

REST (viewer role): `GET /api/storage/volumes`, `GET /api/storage/status`. The web
dashboard exposes a read-only **Storage** page under Resources.

## Local end-to-end

```bash
# 1. Run Atlas with the fake driver (no Ceph/cluster), auth bypassed
cd ../atlas && ATLAS_CEPH_DRIVER_MODE=fake ATLAS_AUTH_REQUIRED=0 make run   # :5110

# 2. Point Aether at it and inspect
AETHER_ATLAS_URL=http://127.0.0.1:5110 aether storage status
```

> Provisioning an actual PVC requires Atlas to reach a Kubernetes cluster (its
> `create_pvc` path calls kube); the fake driver alone serves the read/inventory APIs.
