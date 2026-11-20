---
hero:
  eyebrow: GUIDES
  title: Cilium on Kubernetes with Aether
  tone: violet
  lead: Bootstrap egress policies, workload CNPs, and platform visibility.
---

## What ships today

| Feature | Status |
|---------|--------|
| Cilium CRD deploy from workload spec (`network.ciliumNetworkPolicy`) | **Shipped** |
| Cluster deploy bootstrap (egress CNPs/CCNPs) | **Shipped** |
| API `GET /api/cluster/cilium/status` | **Shipped** |
| Dashboard Platform Cilium card + Clusters **Network** tab | **Shipped** |
| Hubble UI auto-discovery + Fleet deep links | **Shipped** |
| PacketWolf deep integration | **Roadmap** — observe layer stays in PacketWolf |

---

## Bootstrap on cluster deploy

When you deploy Aether to Kubernetes, bootstrap manifests under `deploy/k8s/bootstrap/` apply Aether-managed policies:

**Namespaced (default namespace `aether-system`):**

- `allow-aether-egress` — permissive egress for the control plane
- `allow-aether-egress-strict` / `allow-aether-egress-strict-cluster` — strict variants

**Cluster-wide:**

- `aether-control-plane-egress` (+ strict variants)

Set strict mode before deploy:

```bash
export AETHER_CILIUM_EGRESS_STRICT=1
# re-run cluster deploy / install-cluster.sh from customer bundle
```

The API and Platform page report which bootstrap policies exist via `/api/cluster/cilium/status`.

---

## Workload spec: CiliumNetworkPolicy

Add a Cilium policy block to your workload YAML:

```yaml
network:
  networkPolicy:
    ingress:
      - fromNamespaces: [aether-system]
  ciliumNetworkPolicy:
    ingress:
      - fromEntities: [cluster]
    egress:
      - toEntities: [world]
```

Aether creates:

- `{workload}-netpol` — standard Kubernetes NetworkPolicy (when `networkPolicy` is set)
- `{workload}-cilium` — CiliumNetworkPolicy CR (`cilium.io/v2`)

Implementation: `src/adapters/kube_policy_extras.rs`.

---

## Platform visibility

| Surface | What it shows |
|---------|----------------|
| **Platform → Kubernetes / Cilium** | CNI mode, egress mode, bootstrap checklist, metrics-server |
| **Clusters → Network tab** | NetworkPolicy + Cilium CNPs/CCNPs; `aether-managed` status for bootstrap names |
| **Metrics page** | `/api/observability/summary` — API SLO counters + optional cluster/Cilium context |

Environment variables:

| Variable | Purpose |
|----------|---------|
| `AETHER_PROMETHEUS_URL` | External Prometheus; enables whitelisted proxy queries |
| `AETHER_GRAFANA_URL` | Grafana link on Platform / Metrics |
| `AETHER_GRAFANA_DASHBOARD_UID` | Deep link to imported Aether dashboard |
| `AETHER_HUBBLE_UI_URL` | Optional Hubble UI override; auto-discovery when unset |

Import the bundled Grafana dashboard:

```bash
./scripts/import-grafana-dashboard.sh
```

---

## metrics-server

Cluster CPU/memory in the Clusters browser uses `kubectl top`. On **k3s/kind/minikube/microk8s** deploys, `scripts/deploy-k8s.sh` and `scripts/deploy-remote.sh` install metrics-server automatically when `AETHER_INSTALL_METRICS_SERVER=auto` (default). Force install on any cluster with `AETHER_INSTALL_METRICS_SERVER=1`.

---

## Observability wiring

After Prometheus/Grafana are installed in the cluster:

```bash
eval "$(./scripts/wire-observability-env.sh)"
# optional: GRAFANA_API_KEY=... eval "$(./scripts/wire-observability-env.sh)"
kubectl -n aether-system set env deployment/aether \
  AETHER_PROMETHEUS_URL="${AETHER_PROMETHEUS_URL:-}" \
  AETHER_GRAFANA_URL="${AETHER_GRAFANA_URL:-}"
```

---

## Cilium connectivity check

**Ship:** The control plane runs native probes via `POST /api/cluster/cilium/connectivity/probe` and after Kubernetes deploys. Results are stored in ConfigMap `aether-cilium-connectivity` and surfaced on the Platform page (`connectivity_check: ok|failed|skipped`, with `last_checked_at` and `detail`).

Deploy scripts still run `aether_probe_cilium_connectivity` and optionally apply `deploy/k8s/bootstrap/cilium-connectivity-cronjob.yaml` for periodic checks.

Skip with `AETHER_SKIP_CILIUM_CONNECTIVITY=1`.

---

## Hubble UI

**Ship:** Hubble is auto-discovered from cluster Services/ConfigMaps when installed. Set `AETHER_HUBBLE_UI_URL` to override. Deep links: `{hubble_url}/?namespace={ns}&pod={pod}`.

---

## Helm RBAC

The Aether Helm chart includes `cilium.io` rules so in-cluster API pods can list Cilium policies. See `helm/aether/values.yaml` → `rbac.rules`.

---

## Related docs

- [Networking architecture](../../architecture/NETWORKING.md)
- [Production reference](../../deployment/PRODUCTION-REFERENCE.md)
- [ROADMAP — Hubble / PacketWolf](../../ROADMAP.md)
