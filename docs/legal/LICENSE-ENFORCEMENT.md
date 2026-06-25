# License enforcement (Zeus OS — draft)

**Zeus OS is licensed by managed Kubernetes node count.** The enforcement subsystem reads a signed `.zyvor` license file, counts billable nodes in real time, and transitions the installation through defined states — from valid to blocked — based on expiry and node-count thresholds.

## Overview

| Dimension | Detail |
|-----------|--------|
| Billing unit | Managed Kubernetes node (see Node counting rules) |
| License file format | `.zyvor` JSON envelope with RSA-2048 signature |
| Runtime path | `ZEUS_LICENSE_PATH` env var or default mount |
| Current enforcement | v0.3.0 — warning-only; no blocking |
| Next enforcement | v0.4.0 — grace period + blocking for expired / over-limit |

## License file format

A `.zyvor` file is a JSON document with three top-level fields.

| Field | Type | Description |
|-------|------|-------------|
| `format` | string | Always `"zyvor-v1"` — rejects unknown versions |
| `payload` | string | Base64-encoded JSON object containing all license claims |
| `signature` | string | Base64-encoded RSA-2048 PKCS#1 v1.5 SHA-256 signature over the raw `payload` bytes |

Example envelope:

```json
{
  "format": "zyvor-v1",
  "payload": "<base64-encoded claims JSON>",
  "signature": "<base64-encoded RSA signature>"
}
```

The enforcement subsystem verifies the signature against the embedded Zyvor public key before deserialising the payload. A tampered or self-signed file produces `INVALID_SIGNATURE`.

## License claims fields

The decoded `payload` object carries ten fields.

| Field | Type | Description |
|-------|------|-------------|
| `license_id` | string (UUID) | Globally unique identifier for this license grant |
| `customer_name` | string | Legal entity name of the licensee |
| `product` | string | Must be `"zeus-os"` — any other value yields `WRONG_PRODUCT` |
| `tier` | string | License tier (e.g., `"professional"`, `"enterprise"`, `"sovereign"`) |
| `node_limit` | integer | Maximum billable managed nodes permitted; `-1` means unlimited |
| `issued_at` | integer | Unix epoch seconds — issue timestamp |
| `expires_at` | integer | Unix epoch seconds — hard expiry; `0` means perpetual |
| `grace_days` | integer | Days after `expires_at` before the installation is blocked (e.g., `14`) |
| `features` | string[] | Explicit feature flags granted (e.g., `["multi-tenant","gpu-pools"]`) |
| `metadata` | object | Opaque key-value bag for customer-specific notes; not enforced |

## Node counting rules

Only nodes that satisfy all three conditions below are counted as billable managed nodes.

| Condition | Detail |
|-----------|--------|
| `Ready` | Node must report `Ready=True` in its status conditions |
| Schedulable | `spec.unschedulable` must be absent or `false` — cordoned nodes are excluded |
| Non-control-plane | Nodes bearing `node-role.kubernetes.io/control-plane` or `node-role.kubernetes.io/master` labels are excluded |

Virtual nodes (e.g., Virtual Kubelet / Fargate providers), nodes in `NotReady` state, and nodes explicitly tainted `NoSchedule` with the control-plane taint are never counted. The live count is refreshed every 30 seconds by the background reconciliation loop.

## License states

Nine states are defined. The enforcement subsystem transitions between them on each reconciliation tick.

| State | Trigger condition |
|-------|-------------------|
| `VALID` | Signature valid, product matches, current time is before `expires_at`, node count is at or below `node_limit` |
| `EXPIRING_SOON` | Valid in all other respects, but `expires_at` is within 30 days of now |
| `EXPIRED_GRACE` | Past `expires_at` but within the `grace_days` window; installation continues with warnings |
| `EXPIRED_BLOCKED` | Past `expires_at` + `grace_days`; mutating operations are blocked (v0.4.0+) |
| `OVER_LIMIT_GRACE` | Live node count exceeds `node_limit` by 1–10 nodes; installation continues with warnings |
| `OVER_LIMIT_BLOCKED` | Live node count exceeds `node_limit` by more than 10 nodes; mutating operations are blocked (v0.4.0+) |
| `MISSING` | No license file found at the resolved path; installation runs in warning-only mode |
| `INVALID_SIGNATURE` | File found but signature verification failed (tampered, wrong key, or self-signed) |
| `WRONG_PRODUCT` | Signature valid but `product` claim is not `"zeus-os"` |

States `EXPIRING_SOON` and `OVER_LIMIT_GRACE` are advisory — they never block operations. `MISSING` is treated as warning-only in v0.3.0 and will become blocking in a future major version per the commercial agreement.

## Enforcement policy

| Version | Behaviour |
|---------|-----------|
| v0.3.0 (current) | All states are warning-only — license status is surfaced in API responses, dashboard banner, and structured logs; no request is rejected |
| v0.4.0 (planned) | `EXPIRED_BLOCKED` and `OVER_LIMIT_BLOCKED` reject mutating API calls with HTTP 402; read-only and dashboard paths remain unaffected |
| Future | `MISSING` and `INVALID_SIGNATURE` may block mutating operations; exact version TBD |

**Never blocked regardless of license state:**

- Virtual machine lifecycle operations (KubeVirt, Metal3) — infra workloads must remain reachable
- Read-only API endpoints (`GET` requests, status, metrics)
- Web dashboard rendering and SSE event stream
- `aether license status` / `aether license reload` CLI commands
- Audit log writes

## API reference

### GET /api/license/status

Returns the current license state and key claims.

```
GET /api/license/status
Authorization: Bearer <token>
```

Response `200 OK`:

```json
{
  "state": "EXPIRING_SOON",
  "license_id": "a1b2c3d4-0000-0000-0000-e5f6a7b8c9d0",
  "customer_name": "Acme Corp",
  "tier": "enterprise",
  "node_limit": 50,
  "expires_at": 1782000000,
  "grace_days": 14,
  "days_remaining": 22,
  "features": ["multi-tenant", "gpu-pools"]
}
```

`state` is always present. All other fields are omitted when `state` is `MISSING` or `INVALID_SIGNATURE`.

### GET /api/license/usage

Returns the live billable node count.

```
GET /api/license/usage
Authorization: Bearer <token>
```

Response `200 OK`:

```json
{
  "managed_nodes": 38,
  "node_limit": 50,
  "over_limit": false,
  "last_counted_at": "2026-06-25T10:00:00Z"
}
```

`node_limit` is `-1` for unlimited licenses. `over_limit` is `true` if `managed_nodes > node_limit` and `node_limit != -1`.

### POST /api/license/reload

Hot-reloads the license file from disk without restarting the server. Requires Admin role.

```
POST /api/license/reload
Authorization: Bearer <admin-token>
Content-Length: 0
```

Response `200 OK`:

```json
{
  "state": "VALID",
  "reloaded_at": "2026-06-25T10:05:00Z"
}
```

Response `403 Forbidden` if the caller does not hold the Admin RBAC role.

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ZEUS_LICENSE_PATH` | `/etc/zeus/license.zyvor` | Absolute path to the `.zyvor` license file; overrides the Helm mount path if set at runtime |

No other environment variables affect license enforcement. The signing key is compiled into the binary and cannot be overridden.

## Customer installation guide

The standard installation mounts the license file as a Kubernetes Secret.

**Step 1 — Create the namespace:**

```bash
kubectl create namespace zeus-system
```

**Step 2 — Create the registry pull secret** (skip if using a public registry):

```bash
kubectl create secret docker-registry zyvor-registry \
  --namespace zeus-system \
  --docker-server=registry.zyvor.dev \
  --docker-username=<your-username> \
  --docker-password=<your-token>
```

**Step 3 — Create the license secret:**

```bash
kubectl create secret generic zeus-license \
  --namespace zeus-system \
  --from-file=license.zyvor=/path/to/your/zeus-os.zyvor
```

**Step 4 — Install or upgrade via Helm:**

```bash
helm upgrade --install zeus zyvor/zeus-os \
  --namespace zeus-system \
  --set license.existingSecret=zeus-license \
  --set license.mountPath=/etc/zeus/license.zyvor
```

The Helm chart mounts the secret at `license.mountPath` and sets `ZEUS_LICENSE_PATH` to the same value. No pod restart is needed to reload a replaced secret — use `POST /api/license/reload` after updating the secret data.

## Helm values reference

| Value | Type | Default | Description |
|-------|------|---------|-------------|
| `license.enabled` | bool | `true` | Set to `false` to disable license file mounting entirely (warning-only mode, not for production) |
| `license.existingSecret` | string | `""` | Name of a pre-created `kubernetes.io/opaque` Secret whose key `license.zyvor` holds the license file; takes precedence over `license.secretData` |
| `license.mountPath` | string | `/etc/zeus/license.zyvor` | Absolute path inside the pod where the license file is mounted; must match `ZEUS_LICENSE_PATH` if that variable is set |

Example `values.yaml` fragment:

```yaml
license:
  enabled: true
  existingSecret: zeus-license
  mountPath: /etc/zeus/license.zyvor
```

For air-gapped environments, embed the license data directly:

```yaml
license:
  enabled: true
  secretData: |
    <base64-encoded contents of zeus-os.zyvor>
  mountPath: /etc/zeus/license.zyvor
```

See [LICENSING-MODEL.md](LICENSING-MODEL.md) for tier definitions and commercial metrics. See [PRODUCT-MATRIX.md](PRODUCT-MATRIX.md) for feature gate details.
