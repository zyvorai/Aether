# Aether feature test plan

End-to-end validation for the universal runtime control plane API, React dashboard, labs (Kubernetes / KubeVirt / Metal3), confidential fabric, and Playwright UI — aligned with Zyvor platform staging workflows.

## Quick start

```bash
# Live E2E orchestrator — all live tiers + full Playwright (1–3+ hours)
./scripts/api-live-test.sh <ephemeral-ip> operator

# Quick live preset (~5 min): API walkthrough + smoke only
AETHER_LIVE_TIERS=quick ./scripts/api-live-test.sh <ephemeral-ip> operator

# Makefile remote shortcut
make api-live-test-remote

# Mixed orchestrator (dry-run + smoke; no full API walkthrough)
./scripts/test-all-features-remote.sh <ephemeral-ip> operator

# Smoke only (~2 min)
AETHER_TEST_TIERS=smoke ./scripts/test-all-features-remote.sh HOST USER

# Quick preset
AETHER_TEST_TIERS=quick ./scripts/test-all-features-remote.sh HOST USER

# Full preset (nightly — includes all Playwright specs, 1–3+ hours)
AETHER_TEST_TIERS=full ./scripts/test-all-features-remote.sh HOST USER

# Full Zyvor stack (VMRogue + PacketWolf + Aether)
./scripts/test-zyvor-stack-remote.sh HOST 30151 USER
```

Default API: `http://<host>:30090` (NodePort) · Auth: session cookie via mock IdP, API key, or LDAP when configured.

## Test tiers

| Tier | Script / action | Duration | What it proves |
|------|-----------------|----------|----------------|
| **api-live** | `api-live-test.sh` → `api-live-runner.py` | ~2 min | All routes from `mod.rs`, live terminal output |
| **smoke** | `post-deploy-verify.sh` | ~2 min | Health, 55+ API routes, UX cross-checks, k8s-api-smoke |
| **labs-dry** | `reference-cluster-e2e.sh` | ~1 min | Validate + dry-run Metal3, KubeVirt, k8s lab specs |
| **migration-dry-run** | `migration-dry-run-e2e.sh` | ~30 s | Demo specs validate + `--dry-run` migrate |
| **labs-live** | SSH `k8s-labs-e2e.sh` on remote | ~3–5 min | Live nginx deploy on cluster kubeconfig |
| **confidential** | `confidential-fabric-e2e.sh` + `confidential-cluster-e2e.sh` | ~2 min | Confidential APIs + CLI placement checks |
| **deploy-remove** | `deploy-remove-e2e.sh` | ~2 min | API workload create → verify → delete |
| **playwright-all** | `npm run test:e2e` (entire `tests/`) | 1–3+ hours | All dashboard Playwright specs against remote API |
| **playwright-exec** | `cluster-exec-terminal.spec.ts` | ~5 min | Requires `AETHER_E2E_KIND=1` + kind fixture (local/CI) |

Environment:

| Variable | Purpose |
|----------|---------|
| `AETHER_LIVE_TIERS` | Live orchestrator preset `all` / `quick` or comma list |
| `AETHER_TEST_TIERS` | Comma list or preset `quick` / `full` |
| `AETHER_API` | Base URL (default `http://HOST:30090`) |
| `AETHER_E2E_REPORT_JSON` | Optional tier timing JSON summary |
| `AETHER_REMOTE_HOST` / `AETHER_REMOTE_USER` | SSH target for `labs-live` |
| `AETHER_E2E_SKIP_SERVER` | Set with `AETHER_E2E_BASE_URL` for remote Playwright |
| `AETHER_E2E_KIND` | `1` enables `playwright-exec` tier |
| `AETHER_MOCK_IDP` | Must be `1` on **server** for mock-idp Playwright specs |
| `AETHER_PLAYWRIGHT_TIMEOUT` | Optional Playwright timeout override |

Default live orchestrator tiers (`AETHER_LIVE_TIERS=all`): `api-live,smoke,deploy-remove,confidential,labs-live,playwright-all`.

Default mixed orchestrator tiers (no preset): `smoke,labs-dry,confidential`.

## Feature matrix (API smoke)

### Core platform

| Feature | Tier | Endpoint | Notes |
|---------|------|----------|--------|
| Health | smoke | `GET /health` | No auth |
| Server info | smoke | `GET /api/server` | Cluster connection |
| Readiness | smoke | `GET /api/system/ready` | |
| Auth providers | smoke | `GET /api/auth/providers` | |
| Dashboard version | smoke | `GET /api/dashboard/version` | |

### Workloads & cluster

| Feature | Tier | Endpoint | Notes |
|---------|------|----------|--------|
| Workloads list | smoke | `GET /api/workloads` | Cross-checked with cluster summary |
| Cluster summary | smoke | `GET /api/cluster/summary` | |
| Namespaces / metrics | smoke | `GET /api/cluster/namespaces`, `metrics/summary` | |
| Cilium status / Hubble | smoke | `GET /api/cluster/cilium/status`, `hubble` | |
| Cluster browse | smoke | `GET /api/cluster/browse` | CNP / CCNP |

### Observability & ops

| Feature | Tier | Endpoint | Notes |
|---------|------|----------|--------|
| Observability summary | smoke | `GET /api/observability/summary` | |
| Events / orchestrator | smoke | `GET /api/events`, `orchestrator/summary` | |
| Backups / secrets / plugins | smoke | `GET /api/backups`, `secrets`, `plugins` | |
| GitOps / cost / audit | smoke | `GET /api/gitops/status`, `cost/chargeback` | |
| Confidential fleet | smoke, confidential | `GET /api/confidential/*` | |

### Ecosystem (Four Pillars)

| Feature | Tier | Endpoint | Notes |
|---------|------|----------|--------|
| PacketWolf status | smoke | `GET /api/ecosystem/packetwolf/status` | |
| Security SBOM | smoke | `GET /api/security/sbom` | |
| Fleet edge / federation | smoke | `GET /api/fleet/edge/agents` | |

## Local vs remote vs CI

| Layer | Local command | Remote | CI job |
|-------|---------------|--------|--------|
| Rust unit/integration | `make test` / `cargo test` | — | `test` |
| Schema / confidential validate | `make confidential-validate` | — | `test` |
| Labs dry-run | `scripts/labs-e2e.sh` | `labs-dry` tier | `labs-e2e` |
| Migration dry-run | `scripts/migration-dry-run-e2e.sh` | `migration-dry-run` tier | `migration-dry-run-e2e` |
| K8s live lab | `AETHER_LABS_LIVE=1 k8s-labs-e2e.sh` | `labs-live` tier | `k8s-live-e2e` |
| Live E2E orchestrator | `make api-live-test` | `AETHER_LIVE_TIERS=all ./scripts/api-live-test.sh HOST USER` | Manual / staging |
| API walkthrough | `make api-live-test` (`AETHER_LIVE_TIERS=api-live`) | same | — |
| API smoke | `make post-deploy-verify` | `smoke` tier | `dashboard-e2e` (subset) |
| Playwright subset | `npm run test:e2e -- tests/smoke.spec.ts` | `playwright-all` | `dashboard-e2e` jobs |
| Full Playwright | `npm run test:e2e` | `playwright-all` tier | Nightly only |

## Playwright prerequisites (full suite)

Running `AETHER_TEST_TIERS=full` executes **all** specs in `web/dashboard/tests/` (~82 files).

| Requirement | Specs affected | How to satisfy |
|-------------|----------------|----------------|
| `AETHER_MOCK_IDP=1` on server | `mock-idp-auth.spec.ts` | `scripts/deploy-reference-sso.sh` or env on Deployment |
| `AETHER_MOCK_IDP_ENCRYPTED=1` | Encrypted mock IdP cases | Optional server env |
| `AETHER_E2E_KIND=1` + kind | `cluster-exec-terminal.spec.ts` | Local `playwright-exec` tier only |
| Live cluster metrics | `remote-live-ux.spec.ts` | Deployed API with cluster connected |
| Metal3 / KubeVirt live | Some phase specs | May skip or fail without infra — manual/nightly |

Remote Playwright:

```bash
export AETHER_E2E_SKIP_SERVER=1
export AETHER_E2E_BASE_URL=http://<ephemeral-ip>:30090
cd web/dashboard && npm run test:e2e
```

## Manual-only scenarios

1. **Live migration** — blue-green / rolling across runtimes with real workloads
2. **Metal3 bare metal deploy** — requires Metal3 lab cluster
3. **KubeVirt GPU / SR-IOV** — hardware feature gates
4. **LDAP / OIDC / SAML login** — IdP configured (`deploy-with-ldap.sh`, ingress SSO)
5. **Velero DR / failover** — backup storage configured
6. **Ragnarok composite confidential** — `RAGNAROK_API` set for fabric E2E
7. **SSE reconnect banner** — manual per `web/dashboard/tests/README.md`

## CI recommendation

```bash
make ci                    # fmt, clippy, tests, dashboard glass guard
AETHER_TEST_TIERS=smoke ./scripts/test-all-features-remote.sh HOST USER   # post-deploy staging
```

Nightly / pre-release:

```bash
AETHER_TEST_TIERS=full ./scripts/test-all-features-remote.sh HOST USER
VMROGUE_TEST_TIERS=quick PACKETWOLF_TEST_TIERS=quick AETHER_TEST_TIERS=full \
  ./scripts/test-zyvor-stack-remote.sh HOST 30151 USER
```

GitHub Actions: set `AETHER_STAGING_HOST` and `AETHER_REMOTE_USER` secrets for optional `staging-e2e` job in `.github/workflows/ci.yml`.

## Related scripts

| Script | Role |
|--------|------|
| `scripts/api-live-test.sh` | **Live E2E entry point** — API walkthrough + live tiers + Playwright |
| `scripts/lib/api-live-runner.py` | Route discovery + live terminal API probes |
| `scripts/lib/e2e-tier-runner.sh` | Shared tier runner for orchestrators |
| `scripts/test-all-features-remote.sh` | Laptop orchestrator (quick/full presets, JSON report) |
| `scripts/post-deploy-verify.sh` | Tier smoke |
| `scripts/remote-api-ux-verify.sh` | API + UX consistency |
| `scripts/k8s-api-smoke.sh` | Copilot / observability smoke |
| `scripts/reference-cluster-e2e.sh` | Labs dry-run chain |
| `scripts/remote-reference-verify.sh` | Remote labs + post-deploy |
| `scripts/confidential-fabric-e2e.sh` | Confidential API smoke |
| `scripts/deploy-remove-e2e.sh` | API workload lifecycle |
| `scripts/deploy-remote.sh` | Cluster deploy |
| `./scripts/test-zyvor-stack-remote.sh` | VMRogue + PacketWolf + Aether (delegates to sibling VMRogue) |

## VMRogue / PacketWolf integration

- Smoke tier checks `GET /api/ecosystem/packetwolf/status` when PacketWolf is reachable from the cluster.
- Full platform validation: `./scripts/test-zyvor-stack-remote.sh` (wrapper) or run from the VMRogue repo directly.
