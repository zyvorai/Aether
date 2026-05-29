# Networking Architecture

> How network settings translate across runtimes and what breaks on migrate.

---

## Model

Workload `network` block drives adapter-specific manifests:

| Spec field | Podman | Kubernetes | KubeVirt | Metal3 |
|------------|--------|------------|----------|--------|
| `ports` | `-p host:container` | Service + containerPort | VM service | host networking |
| `ingress` | N/A (local) | Ingress + TLS | Route/Ingress* | Manual |
| `network_policy` | N/A | NetworkPolicy CRD | Limited | N/A |
| `cilium_network_policy` | N/A | CiliumNetworkPolicy CRD | N/A | N/A |
| `service_type` | N/A | ClusterIP/NodePort/LB | Service | N/A |

See [Cilium guide](../guides/kubernetes/CILIUM.md) for bootstrap policies, dashboard visibility, and Hubble auto-discovery.

\* Depends on cluster ingress controller.

---

## Service discovery

| Runtime | Discovery |
|---------|-----------|
| Podman | localhost published ports |
| Kubernetes | Cluster DNS `svc.namespace` |
| KubeVirt | Service → VM port |
| Metal3 | Host IP / BMC management net |

**DNS names do not migrate** — update clients after cutover.

---

## Migration impact

| Concern | On migrate |
|---------|------------|
| Host ports (Podman) | Change — update bookmarks |
| K8s Service ClusterIP | New Service created |
| LoadBalancer IP | Cloud LB reprovisions |
| Ingress hostname | Same if spec unchanged; verify TLS secret |
| NetworkPolicy | Regenerated from spec |

---

## Translation table

```
Podman:  -p 8080:80
    ↓ migrate to K8s
K8s:     Service port 80 → targetPort 80, Ingress optional
    ↓ migrate to KubeVirt
KubeVirt: Service → VM guest port
```

Operators should run `aether validate` and diff manifests (`aether diff`) before production migrate.

---

## Code references

- `src/adapters/podman.rs` — port publish
- `src/adapters/kube.rs` — Service, Ingress, NetworkPolicy
- `src/adapters/kubevirt.rs` — VM network interfaces
- `src/adapters/metal.rs` — host networking
