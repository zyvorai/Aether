# Aether Roadmap

> Ship vs Roadmap — no inflated claims.

---

## Shipped (Trust Layer)

- Product portability spine (README, PRODUCT.md)
- Migration internals + trace mode
- `aether decide --explain` + API/dashboard explain
- Deployment topologies, fleet doc (honest now vs later)
- Stateful portability guide
- Demo folders + bench harness template
- Security, networking, production, cloud matrix docs
- Ecosystem + product tiers
- **Hubble UI discovery** — API `GET /api/cluster/cilium/hubble`, Fleet pod deep links, `AETHER_HUBBLE_UI_URL` override
- **Fleet overview dashboard** — expandable apps, per-pod observability links, cluster resource browser
- **CloudOS Kubernetes UX** — Applications, Activity Monitor, Security Center, Helm App Store, AI troubleshoot panel, workspace selector, Pro View
- **PacketWolf live bridge** — `/api/ecosystem/packetwolf/*`, Fleet + Security Center cards, `AETHER_PACKETWOLF_URL`
- **Edge agent + federation** — `aether edge-agent`, `/api/fleet/edge/*`, `/api/fleet/federation/*`, Fleet Edge/Placement tabs
- **CycloneDX SBOM** — `aether sbom export|verify`, `GET /api/security/sbom`, signed image catalog API
- **SAML encrypted assertions** — AES-128-CBC + RSA-OAEP via `AETHER_SAML_SP_KEY`, mock IdP `AETHER_MOCK_IDP_ENCRYPTED=1`
- **Playwright exec E2E** — `scripts/kind-playwright-fixture.sh`, `cluster-exec-terminal.spec.ts`, CI `dashboard-exec-e2e` job
- **Intelligent Fleet** — anomaly-based placement (PacketWolf), GitOps auto-target via federation, fleet-wide drift API + dashboard
- **Migration depth** — cross-cluster volume replication plans, coordinated fleet migration plans
- **Hosted SaaS foundation** — tenant registry, billing usage API, `X-Aether-Tenant` / `AETHER_TENANT_ID` isolation hooks

---

## Q3–Q4 targets (Roadmap)

| Item | Area | Tag |
|------|------|-----|
| Hosted SaaS billing integration (Stripe) | Product | `hosted` |
| Live volume replication executor (CSI/snapshot) | Migration | `migration` |
| Anomaly auto-remediation workflows | AI | `ai` |

---

## How to read this doc

- **Ship** = in repo today with docs/tests
- **Roadmap** = architecture documented, not claimed in product UI
- **Lab** = `examples/labs/` only

Update this file when items graduate to Ship.

---

## Related engineering docs

- [NEXT-STEPS.md](NEXT-STEPS.md) — HA/OIDC implementation checklist
- [Trust Layer guides](index.md#trust-and-proof)
