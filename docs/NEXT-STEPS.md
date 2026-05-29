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

- **Remote deploy** — `./scripts/deploy-remote.sh 212.8.252.194 sus` (NodePort 30090, Cilium bootstrap, CloudOS UI)
- **Post-deploy verify** — `scripts/post-deploy-verify.sh` (remote API + CloudOS + k8s smoke)
- **Applications restart** — cluster workloads use `/api/cluster/action` restart (not only Aether-managed API)
- **AI troubleshoot Apply fix**: restart, drift reconcile, and rollback wired to cluster/Aether APIs from WorkloadDetail
- **Schema CI**: `scripts/validate-schema-examples.sh` + ajv cross-check in dashboard build; `examples/workload-k8s-advanced.yaml`
- **API smoke**: Helm catalog, Hubble discovery, copilot troubleshoot in `scripts/k8s-api-smoke.sh`
- **Kubernetes labs**: `examples/labs/kubernetes/workload.yaml`, `scripts/k8s-labs-e2e.sh`, CI `k8s-live-e2e` job (kind + live deploy + stop --cascade)
- **SAML SSO**: env-gated SP with `/api/auth/saml/login`, `/api/auth/saml/acs`, dashboard Sign in with SAML
- **Cost CLI JSON**: `aether --output json cost` emits structured `CostComparison` / `CostEstimate`
- **Live pricing fetcher**: `scripts/fetch-pricing.sh` for `AETHER_PRICING_URL` overlays (Azure Retail API when reachable)
- **Playwright**: `clusters-ui.spec.ts` for cluster browser route, SAML/OIDC auth gates, bearer login form
- **OPA enforce** on `POST /api/workloads`, `PUT /api/workloads/:name`, and `POST /api/cluster/apply` when `AETHER_OPA_ENFORCE=true`
- **Alert rules** `ErrorRateAbove` and `CostExceeds` wired to health failure rates and priced fleet/workload costs
- **Cost / chargeback**: `GET /api/cost/pricing`, `GET /api/cost/chargeback`, optional live overlay via `AETHER_PRICING_URL`, regional multiplier via `AETHER_COST_REGION`
- **Dashboard**: chargeback table on Metrics page; Playwright tests for auth, cost, cluster API
- **Labs CI**: `labs-e2e` job + `scripts/labs-e2e.sh`

- **Four Pillars shipped** — PacketWolf bridge, edge agent, federation placement, SBOM, SAML encrypted assertions, Playwright exec fixture
- **Phase A (Intelligent Fleet)** — PacketWolf anomaly placement scoring, GitOps `resolve_deploy_target`, `GET /api/fleet/drift`, Fleet placement anomaly columns
- **Phase B (Migration depth)** — `POST /api/migration/volume/plan`, `POST /api/migration/fleet/plan` for cross-cluster volume + coordinated fleet migrations
- **Phase C (Hosted SaaS)** — `GET/POST /api/hosted/tenants`, `GET /api/hosted/billing/usage`, tenant store at `~/.aether/tenants.json`
- **Phase D (Billing + remediation + volume execute)** — tenant API keys (`/api/hosted/tenants/:id/keys`), request metering (`/api/hosted/billing/metering`), Stripe checkout/webhook, `POST /api/migration/volume/execute`, `GET/POST /api/intelligence/remediation/*`, Hosted dashboard page
- **Edge agent offline replay** — `edge_executor` dispatches gitops/stop/start/restart/delete/drift/cluster actions with local offline queue at `~/.aether/edge-{site}-offline.json`
- **New env vars**: `AETHER_PACKETWOLF_URL`, `AETHER_PACKETWOLF_API_KEY`, `AETHER_EDGE_TOKEN`, `AETHER_FEDERATION_CLUSTERS`, `AETHER_FEDERATION_WEIGHTS`, `AETHER_SAML_SP_KEY`, `AETHER_MOCK_IDP_ENCRYPTED`, `AETHER_E2E_KIND`, `AETHER_STRIPE_SECRET_KEY`, `AETHER_STRIPE_WEBHOOK_SECRET`, `AETHER_STRIPE_PRICE_TEAM`, `AETHER_STRIPE_PRICE_ENTERPRISE`
- **New CLI**: `aether sbom export|verify`, `aether edge-agent --control-plane URL --site NAME`

## Remaining / optional

- **SAML** — additional IdP dialects beyond exc/inclusive C14N and AES-256-GCM (e.g. exclusive with comments only)

## Recommended Next Order

1. Dedicated reference-cluster runner with `AETHER_LABS_LIVE=1` for Metal3/KubeVirt apply smoke — `make reference-cluster-live` / `scripts/labs-live-smoke.sh`
2. Kind fixtures for Playwright cluster exec/port-forward UI tests.
3. Deploy to remote reference cluster and run `scripts/post-deploy-verify.sh`.

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
