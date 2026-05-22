# Next Steps

## Current State

Aether ships a native Kubernetes and KubeVirt cluster browser with exec, port-forward, live watch, and Helm release actions in the dashboard. Production deploy paths support:

- **Ingress + TLS** (`AETHER_EXPOSE`, `AETHER_INGRESS_HOST`, cert-manager annotations)
- **HA workload state** (`AETHER_STATE_DATABASE_URL` + Helm `postgresql.*`)
- **OIDC + Redis sessions** for multi-replica API
- **Remote backup upload** (`AETHER_BACKUP_REMOTE_URL`) and **audit webhook** (`AETHER_AUDIT_WEBHOOK_URL`)
- **Prometheus ServiceMonitor** in Helm (`metrics.serviceMonitor.enabled`)

CI builds the embedded dashboard first, validates example specs (including Metal3/KubeVirt labs), runs Playwright API/UI smoke tests, and runs `scripts/labs-e2e.sh` (validate + dry-run; live deploy when `AETHER_LABS_LIVE=1`).

## Recently completed

- **OPA enforce** on `POST /api/workloads`, `PUT /api/workloads/:name`, and `POST /api/cluster/apply` when `AETHER_OPA_ENFORCE=true`
- **Alert rules** `ErrorRateAbove` and `CostExceeds` wired to health failure rates and priced fleet/workload costs
- **Cost / chargeback**: `GET /api/cost/pricing`, `GET /api/cost/chargeback`, optional live overlay via `AETHER_PRICING_URL`, regional multiplier via `AETHER_COST_REGION`
- **Dashboard**: chargeback table on Metrics page; Playwright tests for auth, cost, cluster API
- **Labs CI**: `labs-e2e` job + `scripts/labs-e2e.sh`

## Remaining / optional

- **Live cluster E2E** in CI (set `AETHER_LABS_LIVE=1` + kubeconfig on a dedicated runner)
- **Playwright UI flows** for in-browser OIDC login, cluster exec terminal, and port-forward (needs IdP + cluster fixtures)
- **SAML / additional IdPs** beyond OIDC (enterprise SSO catalogs)

## Recommended Next Order

1. Dedicated reference-cluster runner with `AETHER_LABS_LIVE=1` for Metal3/KubeVirt apply smoke.
2. Playwright UI tests with mock IdP and kind cluster for exec/port-forward.
3. Optional: wire cloud vendor pricing APIs (AWS Price List, Azure Retail) behind `AETHER_PRICING_URL` fetcher service.

## Notes

- Validate specs with: `aether --spec <file> validate` (global `--spec` before subcommand).
- Embedded UI updates require `cd web/dashboard && npm run build` then `cargo build`.
- Keep `Secret` values redacted in cluster detail responses.
