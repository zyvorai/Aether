# Demo 01: Podman → Kubernetes

Migrate a stateless web workload from local Podman to Kubernetes.

## Prerequisites

- Podman and a Kubernetes cluster (kind/minikube/k3s)
- Aether built: `cargo build --release`

## Workloads

- `workload-podman.yaml` — initial deploy target (Podman)
- `workload-kube.yaml` — same spec with `runtime.preferred: kube`

## Steps

```bash
# 1. Validate
aether validate --spec workload-podman.yaml

# 2. Deploy on Podman
aether run --spec workload-podman.yaml --runtime podman

# 3. Check placement decision
aether decide --spec workload-podman.yaml --explain

# 4. Migrate to Kubernetes
AETHER_MIGRATION_TRACE=1 aether migrate demo-podman-k8s kubernetes --strategy blue-green --verbose-trace

# 5. Verify
aether status demo-podman-k8s
aether logs demo-podman-k8s
```

## Expected output

- Migration summary with strategy `blue-green`
- Target runtime `kubernetes`
- Trace lines: `start-target`, `health-gate`, `drain`, `stop-source`, `update-state`

## Rollback

If migration fails with rollback enabled, source Podman instance is restored. Manual rollback:

```bash
aether migrate demo-podman-k8s podman --strategy immediate
```

## Timings

Record results in `benchmarks/RESULTS.md` after running `scripts/run-demo-migration.sh 01`.
