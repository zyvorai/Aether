# SPIRE workload identity for Ragnarok confidential fabric

Deploy SPIRE to issue SPIFFE SVIDs for confidential workloads. Ragnarok reads the agent socket and surfaces SPIFFE IDs in trust scores.

## Quick install (lab)

```bash
./scripts/install-confidential-spire.sh
./scripts/enable-confidential-production.sh --spire
# Uses SPIFFE CSI driver volume (works with restricted PSS). For hostPath instead:
# ./scripts/enable-confidential-production.sh --spire --mount-agent-socket
```

Manual Helm equivalent:

```bash
helm repo add spiffe https://spiffe.github.io/helm-charts-hardened/
helm upgrade --install --create-namespace -n spire spire-crds spiffe/spire-crds \
  --repo https://spiffe.github.io/helm-charts-hardened/
helm upgrade --install -n spire spire spiffe/spire \
  --repo https://spiffe.github.io/helm-charts-hardened/
```

## Ragnarok configuration

| Variable | Purpose |
|----------|---------|
| `RAGNAROK_SPIRE_ENABLED` | `1` to use SPIRE-backed SPIFFE IDs |
| `SPIRE_AGENT_SOCKET` | Agent socket (default `/run/spire/sockets/agent.sock`) |
| `SPIRE_TRUST_DOMAIN` | Trust domain (default `ragnarok.zyvor.dev`) |
| `RAGNAROK_SPIFFE_ID_<WORKLOAD>` | Per-VM SPIFFE ID override when agent is local to API |

## API

- `GET /api/v1/confidential/spire/status` — agent socket reachability and config
- Trust scores include `spiffe_id` when SPIRE is enabled

## Federation (optional)

Set `RAGNAROK_SPIRE_FEDERATION=1` when federating with Aether trust domain bundles.
