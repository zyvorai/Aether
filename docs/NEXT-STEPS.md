# Next Steps

## Current State

Aether ships a native Kubernetes and KubeVirt cluster browser with exec, port-forward, live watch, and Helm release actions in the dashboard. Production deploy paths support ingress + TLS, HA state, OIDC/Redis sessions, remote backup upload, and Prometheus ServiceMonitor in Helm.

CI builds the embedded dashboard, validates example specs, runs Playwright smoke tests, kind live E2E for cluster exec/port-forward, remote reference verify, and mock IdP SAML flows.

## Recently completed

- **Dashboard phases 2181–2230** — hosted federation panel, hub cross-links (Platform/Fleet/GitOps/Settings/Backups), Playwright `phases-2181-2230-features.spec.ts`
- **Hosted SaaS federation** — `GET /api/hosted/federation`, `POST /api/hosted/tenants/:id/federation/plan`, managed federation UI on Hosted page
- **SAML enterprise dialects** — RSA-SHA384 verification, Azure AD enveloped-signature mock IdP (`AETHER_MOCK_IDP_SHA384`, `AETHER_MOCK_IDP_AZURE_AD`)
- **Metal3 lab MAC** — annotations in `examples/labs/metal3/workload.yaml`, env overrides `AETHER_METAL3_BOOT_MAC` / `AETHER_METAL3_IMAGE_URL`
- **Remote reference verify** — `make remote-reference-verify` (SSH k8s live lab + post-deploy)
- **Post-deploy auth bootstrap** — mock IdP SAML session for verify scripts
- **Kind port-forward E2E** — `cluster-exec-terminal.spec.ts`
- **Remote reference deploy** — `<ephemeral-ip>:30090` with 54/54 post-deploy checks

## Remaining / optional

- **Confidential live** — SEV-SNP hardware or attestation-capable host for live confidential placement
- **Hosted SaaS production** — Stripe production keys on hosted control plane (portal + checkout UI shipped)

## Recently completed (2231–2280)

- **Orchestrator / Edge / Intelligence hub cross-links** — Health, Intelligence, Fleet hub banners
- **Stripe billing portal** — `POST /api/hosted/billing/stripe/portal`, Hosted SaaS checkout + portal buttons
- **Deploy helpers** — `deploy-reference-ingress.sh`, `deploy-reference-sso.sh`, `deploy-with-ldap.sh`, env examples under `examples/deploy/`
- **Settings RBAC link** — Settings hub → Access Control

## Recently completed (2281–2330)

- **Migrations / Security / Cost / Platform / Backups hub cross-links** — FinOps and migration navigation banners
- **Migration dry-run CI** — `scripts/migration-dry-run-e2e.sh`, CI `migration-dry-run-e2e` job, `make migration-dry-run-e2e`
- **Stripe production deploy** — `examples/deploy/stripe-production.env.example`, `deploy-hosted-stripe-prod.sh`
- **Confidential SNP lab** — `examples/deploy/confidential-snp.env.example`, `deploy-confidential-snp-lab.sh`
- **Playwright** — `phases-2281-2330-features.spec.ts`

## Recently completed (live E2E orchestrator)

- **Live E2E orchestrator** — `scripts/api-live-test.sh` runs all live tiers (API walkthrough, smoke, deploy-remove, confidential, labs-live, Playwright) with shared tier runner in `scripts/lib/e2e-tier-runner.sh`
- **Makefile** — `make api-live-test`, `make api-live-test-remote`
- **Presets** — `AETHER_LIVE_TIERS=all|quick`; see `docs/TEST_PLAN.md`

## Recommended Next Order

1. **Dashboard phases 2331+** — next hub slice
2. **Confidential live** when SNP lab host is available
3. **Ingress + TLS** on reference cluster (`AETHER_EXPOSE=ingress` via `deploy-reference-ingress.sh`)
4. **Hosted SaaS production** — set live Stripe keys via `deploy-hosted-stripe-prod.sh`

## Mock IdP (development / CI)

```bash
AETHER_MOCK_IDP=1 AETHER_SESSION_SECRET=mock-idp-dev-session-key-32chars \
  cargo run -- serve --host 127.0.0.1 --port 5090
```

Dialect flags:

- `AETHER_MOCK_IDP_EXCLUSIVE_COMMENTS=1` — exc-c14n WithComments
- `AETHER_MOCK_IDP_SHA384=1` — RSA-SHA384 signatures
- `AETHER_MOCK_IDP_AZURE_AD=1` — Azure AD enveloped-signature transforms

Hub cross-links E2E:

```bash
npm run test:e2e -- tests/phases-2181-2230-features.spec.ts
```

Remote verify:

```bash
make remote-reference-verify
AETHER_API=http://<ephemeral-ip>:30090 make post-deploy-verify
make api-live-test-remote   # full live E2E (API + Playwright; 1–3+ hours)
AETHER_LIVE_TIERS=quick make api-live-test-remote   # API walkthrough + smoke (~5 min)
```

## Notes

- Validate specs with: `aether --spec <file> validate`
- Embedded UI updates require `cd web/dashboard && npm run build` then `cargo build`
- On macOS, do not use `deploy-remote --local-build` for Linux remotes (use remote cargo)
