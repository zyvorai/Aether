# Metal3 reference lab

Minimal checklist for exercising Aether bare-metal runtime selection against a Metal3 dev cluster.

## Prerequisites

- Bare-metal hosts registered in Metal3 (`BareMetalHost` CRs ready)
- `kubectl` context pointing at the management cluster
- Aether built with `cargo build --release`

## Quick validation

```bash
export KUBECONFIG=~/.kube/config
aether validate --spec examples/metal3-workload.yaml
aether run --spec examples/metal3-workload.yaml --runtime metal --dry-run
```

## Dashboard

Open **Workloads** → deploy a Metal-tagged spec → confirm runtime **metal3** in the detail panel.

## Troubleshooting

- Ensure `BareMetalHost` objects show `provisioning` / `provisioned` state before deploy.
- High resource requests trigger the engine’s Metal3 path — see workload `requirements` in the spec.
