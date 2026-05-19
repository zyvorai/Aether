# Next Steps

## Current State

Aether ships a native Kubernetes and KubeVirt cluster browser with exec, port-forward, live watch, and Helm release actions in the dashboard. Production deploy paths support:

- **Ingress + TLS** (`AETHER_EXPOSE`, `AETHER_INGRESS_HOST`, cert-manager annotations)
- **HA workload state** (`AETHER_STATE_DATABASE_URL` + Helm `postgresql.*`)
- **OIDC + Redis sessions** for multi-replica API
- **Remote backup upload** (`AETHER_BACKUP_REMOTE_URL`) and **audit webhook** (`AETHER_AUDIT_WEBHOOK_URL`)
- **Prometheus ServiceMonitor** in Helm (`metrics.serviceMonitor.enabled`)

## Remaining Work

- Owner-reference graph in cluster detail (beyond audit name filter)
- **OPA bundle sync** sidecar / ConfigMap reload in Helm
- **Real cost/chargeback** integrations (cloud pricing APIs)
- **Dashboard E2E** tests (exec WS, port-forward, OIDC login)
- **Metal3 / KubeVirt** install guides and hardware e2e on reference clusters

## Recommended Next Order

1. Event correlation panel on cluster resource detail.
2. OPA bundle sync + policy deny in API.
3. Playwright smoke tests for dashboard auth and cluster browser.
4. Reference Metal3/KubeVirt lab manifests.

## Notes

- Keep `Secret` values redacted in cluster detail responses.
- Prefer expanding the native Aether backend and UX instead of reintroducing Headlamp dependencies.
