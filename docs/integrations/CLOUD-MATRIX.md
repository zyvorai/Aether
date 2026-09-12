# Cloud Integration Matrix

> Vendor support levels for Aether target runtimes.

| Vendor | Runtime | Level | Notes |
|--------|---------|-------|-------|
| AWS EKS | Kubernetes | **Supported** | kubeconfig + standard adapter |
| Google GKE | Kubernetes | **Supported** | See `examples/labs/` |
| Azure AKS | Kubernetes | **Supported** | See `examples/labs/` |
| OpenStack | KubeVirt/K8s | **Lab** | Tenant clusters via kubeconfig |
| Proxmox | KubeVirt | **Lab** | VM bridge networking manual |
| On-prem K8s | Kubernetes | **Supported** | Primary enterprise path |
| Local Podman | Podman | **Supported** | Dev and edge |

**Levels:** Supported = documented + tested paths · Lab = examples/labs only · Planned = roadmap

---

## Labs

- Repo: `examples/labs/`
- Docs: `docs/examples/labs/`

Each lab README lists prerequisites and honest limitations.

---

## Cloud-specific notes

### EKS / GKE / AKS

- Use `AETHER_NAMESPACE` or spec namespace override
- LoadBalancer Ingress annotations vary by cloud — set in spec `ingress` block
- GPU: device plugin must exist before KubeVirt/GPU workloads

### OpenStack

- Provide kubeconfig from Magnum or self-managed K8s
- Floating IPs are **not** migrated by Aether

### Proxmox

- KubeVirt on K8s atop Proxmox VMs is the supported path
- Direct Proxmox API integration: **Planned**

---

## Related

- [ROADMAP.md](../ROADMAP.md)
- [COST.md](../guides/operations/COST.md) — provider estimates
