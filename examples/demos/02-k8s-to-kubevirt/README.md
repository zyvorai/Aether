# Demo 02: Kubernetes → KubeVirt

Move a GPU/isolation-oriented workload from Kubernetes to KubeVirt VM runtime.

## Prerequisites

- Kubernetes cluster with KubeVirt installed
- Aether with kube + kubevirt adapters configured

## Steps

```bash
aether validate --spec workload-kube.yaml
aether run --spec workload-kube.yaml --runtime kubernetes
aether decide --spec workload-kube.yaml --explain
aether migrate demo-k8s-kubevirt kubevirt --strategy blue-green
aether status demo-k8s-kubevirt
```

## Expected output

- Scoring favors KubeVirt when GPU or isolation intent is set
- Migration completes with health gate before blue teardown

## Rollback

```bash
aether migrate demo-k8s-kubevirt kubernetes --strategy immediate
```

## Notes

- VM disk data is **not** copied automatically — see [Stateful Portability](../../../docs/guides/migration/STATEFUL-PORTABILITY.md)
