# Aether example workloads

Sample YAML specs for validation, demos, and feature walkthroughs.

## Confidential computing (Ragnarok + Aether)

| File | Runtime | Use case |
|------|---------|----------|
| [confidential-snp.yaml](confidential-snp.yaml) | KubeVirt | SEV-SNP VM with strict attestation and measured image |
| [confidential-kata-clh-snp.yaml](confidential-kata-clh-snp.yaml) | Kata (CoCo) | Reference **Pod** manifest for `kata-clh-snp` (apply with kubectl) |
| [confidential-kata-clh-gpu-snp.yaml](confidential-kata-clh-gpu-snp.yaml) | Kata | Reference **Pod** for `kata-clh-gpu-snp` GPU confidential containers |
| [confidential-migrate-kubevirt.yaml](confidential-migrate-kubevirt.yaml) | KubeVirt | Encrypted live migration plan + `confidential-blue-green` migrate |

Quick start:

```bash
aether validate --spec examples/confidential-snp.yaml
aether --spec examples/confidential-migrate-kubevirt.yaml confidential migration plan --target kubevirt
aether --spec examples/confidential-snp.yaml confidential placement
aether migrate confidential-migrate-demo --target kubevirt --strategy confidential-blue-green
```

See [RAGNAROK-AND-AETHER.md](../docs/guides/security/RAGNAROK-AND-AETHER.md) for CLI ↔ dashboard mapping and phase reference.

Cluster validation: [CONFIDENTIAL-CLUSTER-E2E.md](../docs/guides/security/CONFIDENTIAL-CLUSTER-E2E.md) (`scripts/confidential-cluster-e2e.sh`).

## General

| File | Notes |
|------|-------|
| [workload-full-featured.yaml](workload-full-featured.yaml) | ConfigMaps, Secrets, Ingress, HPA |
| [demo-webserver.yaml](demo-webserver.yaml) | Minimal web service |

Demos with scripts: [demos/](demos/)
