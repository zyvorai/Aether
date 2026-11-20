---
hero:
  eyebrow: EXAMPLES & LABS
  title: KubeVirt reference lab
  tone: teal
---

Checklist for GPU / VM-style workloads via Aether’s KubeVirt adapter.

## Prerequisites

- KubeVirt operator installed on the target cluster
- CDI available if using `DataVolume`
- `kubectl` access to VM namespaces

## Quick validation

```bash
aether validate --spec examples/kubevirt-gpu.yaml
aether run --spec examples/kubevirt-gpu.yaml --runtime kubevirt --dry-run
```

## Dashboard

Use **Cluster Browser** → filter **VirtualMachine** / **VirtualMachineInstance** to inspect VM objects after deploy.

## Intent engine

Workloads with GPU requirements and isolation intent should score toward **kubevirt** in **AI Engine** → Intent Debugger.
