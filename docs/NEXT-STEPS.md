# Next Steps

## Current State

Aether ships a native Kubernetes and KubeVirt cluster browser with exec, port-forward, live watch, and Helm release actions in the dashboard. Production deploy paths support:

- **Ingress + TLS** (`AETHER_EXPOSE`, `AETHER_INGRESS_HOST`, cert-manager annotations)
- **HA workload state** (`AETHER_STATE_DATABASE_URL` + Helm `postgresql.*`)
- **OIDC + Redis sessions** for multi-replica API
- **Remote backup upload** (`AETHER_BACKUP_REMOTE_URL`) and **audit webhook** (`AETHER_AUDIT_WEBHOOK_URL`)
- **Prometheus ServiceMonitor** in Helm (`metrics.serviceMonitor.enabled`)

CI builds the embedded dashboard first, validates example specs (including Metal3/KubeVirt/Kubernetes labs), runs Playwright API/UI smoke tests, `scripts/labs-e2e.sh` (validate + dry-run), and a **kind live E2E** job for Kubernetes.

## Recently completed

- **Kubernetes labs**: `examples/labs/kubernetes/workload.yaml`, `scripts/k8s-labs-e2e.sh`, CI `k8s-live-e2e` job (kind + live deploy)
- **SAML SSO**: env-gated SP with `/api/auth/saml/login`, `/api/auth/saml/acs`, dashboard Sign in with SAML
- **Cost CLI JSON**: `aether --output json cost` emits structured `CostComparison` / `CostEstimate`
- **Live pricing fetcher**: `scripts/fetch-pricing.sh` for `AETHER_PRICING_URL` overlays (Azure Retail API when reachable)
- **Playwright**: `clusters-ui.spec.ts` for cluster browser route, SAML/OIDC auth gates, bearer login form
- **OPA enforce** on `POST /api/workloads`, `PUT /api/workloads/:name`, and `POST /api/cluster/apply` when `AETHER_OPA_ENFORCE=true`
- **Alert rules** `ErrorRateAbove` and `CostExceeds` wired to health failure rates and priced fleet/workload costs
- **Cost / chargeback**: `GET /api/cost/pricing`, `GET /api/cost/chargeback`, optional live overlay via `AETHER_PRICING_URL`, regional multiplier via `AETHER_COST_REGION`
- **Dashboard**: chargeback table on Metrics page; Playwright tests for auth, cost, cluster API
- **Labs CI**: `labs-e2e` job + `scripts/labs-e2e.sh`

## Remaining / optional

- **Playwright in-browser exec terminal** with kind cluster fixtures on a dedicated runner
- **SAML encrypted assertions** and full exclusive-C14N for all IdP XML dialects

## Recommended Next Order

1. Kind fixtures for Playwright cluster exec/port-forward UI tests.
2. Dedicated reference-cluster runner with `AETHER_LABS_LIVE=1` for Metal3/KubeVirt apply smoke.
3. SAML encrypted assertions for production IdPs requiring WSS.

## Mock IdP (development / CI)

Set `AETHER_MOCK_IDP=1` when running `aether serve` to embed a SAML + OIDC test IdP:

```bash
AETHER_MOCK_IDP=1 AETHER_SESSION_SECRET=mock-idp-dev-session-key-32chars \
  cargo run -- serve --host 127.0.0.1 --port 5090
```

Playwright: `npm run test:e2e -- tests/mock-idp-auth.spec.ts` with `AETHER_MOCK_IDP=1` on the server.

SAML signature verification: set `AETHER_SAML_IDP_CERT` to the IdP PEM (auto-set by mock IdP).

## Notes

- Validate specs with: `aether --spec <file> validate` (global `--spec` before subcommand).
- Embedded UI updates require `cd web/dashboard && npm run build` then `cargo build`.
- Keep `Secret` values redacted in cluster detail responses.
