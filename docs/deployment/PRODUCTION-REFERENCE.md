---
hero:
  eyebrow: DEPLOYMENT
  title: Production Reference Architecture
  tone: emerald
  lead: Scale, upgrades, recovery, and controller failure handling.
---

## Reference stack

| Tier | Components |
|------|------------|
| Edge | Ingress (nginx/traefik), TLS, WAF optional |
| Control | 3× `aether serve` pods, anti-affinity |
| Data | PostgreSQL (state), Redis (sessions) |
| Targets | EKS/GKE/AKS/on-prem via kubeconfig |
| Observability | Prometheus scrape `/metrics`, structured JSON logs |

Install via `helm install aether ./helm/aether` with HA values.

---

## Sizing (starting point)

| Workloads | API replicas | Postgres |
|-----------|--------------|----------|
| < 100 | 2 | 2 vCPU / 4Gi |
| 100–500 | 3 | 4 vCPU / 8Gi |
| 500+ | 3+ horizontal | Managed RDS + read replica |

API rate limit: 200 concurrent requests (Tower middleware).

---

## Upgrades

1. Backup state: `aether backup` or Postgres snapshot
2. Rolling update Helm release
3. Verify `/api/health` and dashboard SSE
4. Run smoke: `aether list`, sample migrate dry-run

---

## Controller failure recovery

| Failure | Recovery |
|---------|----------|
| API pod crash | Kubernetes restarts pod; state in Postgres |
| Postgres loss | Restore from backup; replay GitOps |
| Redis loss | Sessions cleared; users re-login |
| Full cluster loss | Restore Postgres + redeploy Helm; workloads re-run from specs |

Workload **runtime** state lives on target clusters — Aether control plane loss does not delete Pods/VMs.

---

## Backup cadence

- State DB: daily automated snapshot
- `~/.aether` (dev): copy off-host weekly
- GitOps repo: source of truth for specs

See [RUNBOOK.md](../guides/operations/RUNBOOK.md), [BACKUP.md](../guides/operations/BACKUP.md).

---

## Observability

| Tier | Components |
|------|------------|
| Metrics | Prometheus scrape `/api/metrics`; set `AETHER_PROMETHEUS_URL` on API pods |
| Dashboards | Grafana + `scripts/import-grafana-dashboard.sh`; set `AETHER_GRAFANA_URL` and `AETHER_GRAFANA_DASHBOARD_UID` |
| Discovery | `eval "$(./scripts/wire-observability-env.sh)"` after installing a monitoring stack |
| Cilium | Bootstrap policies via deploy; connectivity ConfigMap `aether-cilium-connectivity` |

On k3s/kind deploys, `AETHER_INSTALL_METRICS_SERVER=auto` (default) installs metrics-server when missing. See [CILIUM.md](../guides/kubernetes/CILIUM.md).

---

## Related

- [Deployment Topologies](../architecture/DEPLOYMENT-TOPOLOGIES.md)
- [AIR-GAPPED.md](AIR-GAPPED.md)
