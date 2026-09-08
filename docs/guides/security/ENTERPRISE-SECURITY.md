# Enterprise Security

> Production security narrative for regulated environments.

See also: [Security Features](../features/security.md) · [Presentation 04](../../client-presentations/04-security-compliance.html)

---

## Identity and access

| Layer | Capability | Status |
|-------|------------|--------|
| API | Bearer `AETHER_API_KEY` | Ship |
| API | RBAC Admin/Operator/Viewer | Ship |
| Dashboard | OIDC / SAML SSO | Ship (Helm + NEXT-STEPS) |
| CLI | Key from env / config | Ship |

**Production:** Disable long-lived shared keys; use RBAC keys with rotation and SSO for dashboard.

---

## Isolation comparison

| Control | Podman | Kubernetes | KubeVirt |
|---------|--------|------------|----------|
| Process isolation | Namespaces/cgroups | Pod boundary | VM boundary |
| Network policy | Limited | NetworkPolicy | VM + cluster policy |
| Secrets | Files/env | K8s Secrets + encryption at rest | VM secrets |
| Compliance attest | Host OS | PSA/OPA | VM isolation |

---

## Data protection

- AES-256-GCM for secrets at rest (`AETHER_SECRET_KEY`)
- Audit log SHA-256 integrity chain
- Backups/snapshots `0o600`
- TLS termination at Ingress (HA chart)

---

## Compliance roadmap

| Item | Status |
|------|--------|
| SBOM per release | Roadmap |
| Signed workload images | Roadmap |
| Admission controller bundle | Roadmap |
| FIPS-validated crypto module | Roadmap |

Tag issues: `security`, `compliance` in [ROADMAP.md](../ROADMAP.md).

---

## Air-gapped

See [AIR-GAPPED.md](../deployment/AIR-GAPPED.md) for offline bundle install without external registries.

---

## IdP runbook

1. Deploy mock or corporate IdP (Keycloak, Okta) per `docs/NEXT-STEPS.md`
2. Configure Helm `oidc.*` values
3. Map groups → RBAC roles
4. Verify dashboard login and API token exchange

---

## Incident response

- Export audit: `GET /api/audit` or dashboard Audit page
- Revoke key: `POST /api/rbac/keys/revoke`
- Rotate secret key: documented in security.md with re-encrypt steps
