# 🔐 Security Features

> Encryption, secrets management, RBAC, API authentication, policy enforcement, audit integrity, email notifications, and safe state persistence.

---

## 📑 Table of Contents

- [AES-256-GCM Encryption](#-aes-256-gcm-encryption)
- [Setting the Encryption Key](#-setting-the-encryption-key)
- [RBAC API Integration](#-rbac-api-integration)
- [API Authentication](#-api-authentication)
- [CORS Protection](#-cors-protection)
- [Input Validation](#-input-validation)
- [Secret Lifecycle](#-secret-lifecycle)
- [Audit Trail Integrity](#-audit-trail-integrity)
- [File Permission Hardening](#-file-permission-hardening)
- [Policy Engine](#-policy-engine)
- [Policy Gate on Deploy](#-policy-gate-on-deploy)
- [Confirmation Safety](#-confirmation-safety)
- [State File Locking](#-state-file-locking)
- [Atomic State Persistence](#-atomic-state-persistence)
- [Kubernetes Resource Safety](#-kubernetes-resource-safety)
- [Rate Limiting](#-rate-limiting)
- [Structured JSON Logging](#-structured-json-logging)
- [Webhook Delivery Security](#-webhook-delivery-security)
- [Email SMTP Notifications](#-email-smtp-notifications)
- [Cross-References](#-cross-references)

---

## 🔒 AES-256-GCM Encryption

aether encrypts all secret values at rest using **AES-256-GCM** (Galois/Counter Mode), a widely trusted authenticated encryption algorithm.

### How It Works

```
User Key (any length)
        │
        ▼
┌───────────────────┐
│   SHA-256 Hash    │  Derives a fixed 32-byte key from your key material
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Random 12-byte   │  Generated per encryption operation via OsRng
│     Nonce         │  (cryptographically secure)
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  AES-256-GCM      │  Authenticated encryption
│  Encrypt          │  Produces ciphertext + authentication tag
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  nonce ‖ cipher   │  Nonce prepended to ciphertext
│  base64-encode    │  Stored as a single base64 string
└───────────────────┘
```

### Key Properties

| Property | Detail |
|---|---|
| **Algorithm** | AES-256-GCM (authenticated encryption with associated data) |
| **Key derivation** | SHA-256 hash of user-provided key → 32-byte AES key |
| **Nonce** | Random 12-byte nonce generated per encryption via `OsRng` |
| **Output format** | `base64(nonce ‖ ciphertext ‖ auth_tag)` |
| **Nonce uniqueness** | Same plaintext encrypted twice produces **different** ciphertexts |
| **Integrity** | GCM authentication tag detects tampering |
| **Wrong key** | Decryption fails: `"AES-256-GCM decryption failed (wrong key or corrupted data)"` |
| **Short ciphertext** | Rejected: `"Invalid AES ciphertext: too short (missing nonce)"` |

### Development Fallback

When `AETHER_SECRET_KEY` is **not set**, aether derives a **deterministic key from machine identity** (hostname/username) and logs a warning. This key is stable across restarts but is discoverable by other users on the same machine.

```
⚠️  AETHER_SECRET_KEY not set — generating deterministic key from machine identity.
    Set AETHER_SECRET_KEY for stable production encryption.
```

All encryption uses **AES-256-GCM** regardless of key source. The legacy `Obfuscate` (XOR) method is no longer supported — attempting to decrypt XOR-encrypted values produces a clear upgrade message.

### Encryption Methods

| Method | When Used | Security Level | Icon |
|---|---|---|---|
| `Aes256` | Always (both explicit key and machine-derived key) | 🟢 Production-grade | 🔒 |
| `Obfuscate` | Legacy (rejected on decrypt with upgrade instructions) | 🔴 Deprecated | ⚠️ |
| `VaultRef` | External vault reference (not stored locally) | 🟢 Delegated to vault | 🏛️ |

The `SecretValue.method` field tracks which encryption was used per key, allowing mixed-method storage within a single secret.

---

## 🔑 Setting the Encryption Key

Set the `AETHER_SECRET_KEY` environment variable to enable AES-256-GCM encryption.

```bash
# Set for the current session
export AETHER_SECRET_KEY="my-production-secret-key-at-least-16-chars"

# Or in your shell profile (~/.bashrc, ~/.zshrc)
echo 'export AETHER_SECRET_KEY="your-key-here"' >> ~/.bashrc

# Or per-command
AETHER_SECRET_KEY="key" aether secrets set db-creds password "s3cret"
```

### Key Requirements

| Requirement | Detail |
|---|---|
| **Minimum length** | 16 bytes minimum enforced. Keys shorter than 16 bytes produce a warning. |
| **Recommended length** | 32+ characters for maximum entropy |
| **Consistency** | Same key must be used for both encryption and decryption |
| **Rotation** | Changing the key invalidates all previously encrypted values |
| **Empty string** | Falls back to machine-derived key (logged warning) |
| **Character set** | Any UTF-8 bytes are accepted (hashed via SHA-256 to 32 bytes) |

### Production Recommendations

| Recommendation | Why |
|---|---|
| Store the key in a secrets manager (HashiCorp Vault, AWS Secrets Manager) | Avoid plaintext keys on disk |
| Use different keys per environment | Limit blast radius of key compromise |
| Rotate the key periodically | Limit exposure window |
| Never commit the key to source control | Prevents credential leakage |
| Use at least 32 characters | Maximizes entropy before hashing |

---

## 🔄 Secret Lifecycle

### Create a Secret

```bash
aether secrets create db-credentials --namespace production
```

Creates an empty secret container with a **default rotation policy**:

| Policy Field | Default Value |
|---|---|
| `interval_days` | 90 |
| `max_age_days` | 365 |
| `notify_before_days` | 14 |

---

### Set a Key-Value Pair

```bash
aether secrets set db-credentials password "s3cret123"
aether secrets set db-credentials username "admin"
```

Each `set` operation:

1. ✅ Encrypts the value using the active encryption method (AES-256-GCM or XOR fallback)
2. ✅ Increments the key's **version number** (starts at 1)
3. ✅ Updates the secret's `updated_at` timestamp
4. ✅ Logs a `WRITE` event to the access audit trail

If the key already exists, its value is updated and the version increments.

---

### Get a Decrypted Value

```bash
aether secrets get db-credentials password
# Output: s3cret123
```

Each `get` operation:

1. Looks up the secret and key
2. Decrypts using the method recorded in `SecretValue.method`
3. Logs a `READ` event to the access audit trail

---

### Rotate a Secret Value

```bash
aether secrets set db-credentials password "new-s3cret-456"
```

Rotation is equivalent to a `set` with the same key name. The operation:

1. Encrypts the new value
2. Increments the version number
3. Updates `last_rotated` timestamp
4. Logs a `ROTATE` event

---

### Delete a Key

Removing a specific key within a secret:

1. Removes the key from the secret's data map
2. Updates the `updated_at` timestamp
3. Logs a `DELETE` event to the access audit trail

---

### Delete an Entire Secret

Removing the entire secret container removes all keys, the rotation policy, and the access log.

---

### Audit Rotation Status

```bash
aether secrets audit
```

Checks all secrets against their rotation policies and produces alerts:

| Severity | Condition | Message Example |
|---|---|---|
| ⚠️ **Warning** | Key age exceeds `max_age_days - notify_before_days` | `"Key 'password' in 'db-creds' expires in 10 days"` |
| 🔴 **Critical** | Key age exceeds `max_age_days` | `"Key 'password' in 'db-creds' is 400 days old (max: 365)"` |

---

### List All Secrets

```bash
aether secrets list
```

Displays a table with name, namespace, key count, last updated, and rotation status. **Values are never exposed in list output.**

```
╭──────────────┬────────────┬──────┬─────────────────────┬──────────╮
│ Name         │ Namespace  │ Keys │ Updated             │ Rotation │
├──────────────┼────────────┼──────┼─────────────────────┼──────────┤
│ db-creds     │ production │ 2    │ 2026-01-15T10:00:00 │ OK       │
│ api-keys     │ staging    │ 3    │ 2026-01-10T08:30:00 │ ⚠ Yes    │
╰──────────────┴────────────┴──────┴─────────────────────┴──────────╯
```

---

### Access Audit Trail

Every secret operation is logged in the secret's `access_log`:

| Field | Type | Description |
|---|---|---|
| `timestamp` | string (RFC 3339) | When the access occurred |
| `action` | SecretAction | `READ`, `WRITE`, `DELETE`, or `ROTATE` |
| `key` | string | Which key was accessed |
| `actor` | string | Who performed the action (`"cli"` for CLI, custom for API via `*_with_actor()` methods) |

The `*_with_actor()` variants (`set_with_actor`, `get_and_log_with_actor`, `delete_key_with_actor`, `rotate_with_actor`) allow the API server and other callers to pass the real actor identity for proper audit attribution.

---

## 🛂 RBAC API Integration

Aether supports **role-based access control (RBAC)** for API keys via the `RbacStore`. Each API key is assigned one of three roles that determine which operations are permitted.

### Roles

| Role | Permissions | Description |
|---|---|---|
| **Admin** | Full access | Create/revoke keys, mutate workloads, read all data |
| **Operator** | Read + mutate workloads | Start, stop, delete, migrate workloads; cannot manage RBAC keys |
| **Viewer** | Read-only | List workloads, view logs, check health; no mutations |

### API Endpoints

| Method | Path | Description | Required Role |
|---|---|---|---|
| `GET` | `/api/rbac/keys` | List all RBAC keys (names and roles, no raw keys) | Admin |
| `POST` | `/api/rbac/keys` | Create a new RBAC key with a specified role | Admin |
| `POST` | `/api/rbac/keys/revoke` | Revoke an existing RBAC key | Admin |

### How It Works

The API middleware checks incoming requests in this order:

1. **RBAC key check** -- If the `Authorization: Bearer <key>` header matches a key in the `RbacStore`, the request is authorized with the corresponding role.
2. **Fallback to `AETHER_API_KEY`** -- If no RBAC key matches but `AETHER_API_KEY` is set and the Bearer token matches, the request is authorized with Admin-equivalent access. This preserves backward compatibility.
3. **No auth configured** -- If neither RBAC keys nor `AETHER_API_KEY` are configured, all endpoints are public (local development mode).

### Creating RBAC Keys

```bash
# Create an admin key
curl -X POST http://localhost:5090/api/rbac/keys \
  -H "Authorization: Bearer <admin-key>" \
  -H "Content-Type: application/json" \
  -d '{"name": "ci-pipeline", "role": "operator"}'

# List all keys
curl http://localhost:5090/api/rbac/keys \
  -H "Authorization: Bearer <admin-key>"

# Revoke a key
curl -X POST http://localhost:5090/api/rbac/keys/revoke \
  -H "Authorization: Bearer <admin-key>" \
  -H "Content-Type: application/json" \
  -d '{"name": "ci-pipeline"}'
```

### Endpoint Authorization Matrix

| Endpoint Category | Admin | Operator | Viewer |
|---|---|---|---|
| List/get workloads, logs, health | Yes | Yes | Yes |
| Start/stop/delete/migrate workloads | Yes | Yes | No |
| RBAC key management | Yes | No | No |
| Secrets management | Yes | Yes | No |
| Backup/restore | Yes | Yes | No |

---

## 🔑 API Authentication

The REST API server supports **Bearer token authentication** via the `AETHER_API_KEY` environment variable.

### Setup

```bash
# Set an API key to enable authentication
export AETHER_API_KEY="my-secret-api-key-at-least-32-chars"

# Start the server
aether serve
```

### How It Works

| Scenario | Behavior |
|---|---|
| `AETHER_API_KEY` is set and non-empty | All `/api/*` endpoints require `Authorization: Bearer <key>` header |
| `AETHER_API_KEY` is not set or empty | All endpoints are public (local development mode) |
| `/health` and `/` (dashboard) | Always public, no authentication required |
| Invalid or missing token | `401 Unauthorized` response |

### Example API Call

```bash
# With authentication
curl -H "Authorization: Bearer my-secret-api-key" \
     http://localhost:5090/api/workloads

# Health check (always public)
curl http://localhost:5090/health
```

### Startup Messages

```
🌐 Starting API server on http://127.0.0.1:5090
📊 Dashboard: http://127.0.0.1:5090
📋 API Health: http://127.0.0.1:5090/health
🔐 API authentication enabled (AETHER_API_KEY)
```

Without authentication:

```
⚠️  No AETHER_API_KEY set — API is unauthenticated. Set AETHER_API_KEY for production use.
```

---

## 🌐 CORS Protection

The API server restricts Cross-Origin Resource Sharing (CORS) to the server's own origin.

| Setting | Value |
|---|---|
| **Allowed origin** | `http://{host}:{port}` (e.g., `http://127.0.0.1:5090`) |
| **Allowed methods** | `GET`, `POST`, `DELETE` |
| **Allowed headers** | Any |

This prevents cross-origin attacks from untrusted browser contexts while allowing the built-in dashboard to function.

---

## 🛡️ Input Validation

### Workload Names (API)

API endpoints validate workload names against **DNS-1123** format:

| Rule | Detail |
|---|---|
| **Characters** | Lowercase alphanumeric (`a-z`, `0-9`), hyphens (`-`), dots (`.`) |
| **Length** | 1–253 characters |
| **Start/end** | Must start and end with alphanumeric |
| **Rejected** | Uppercase, spaces, `/`, `\`, `..`, `\0`, special characters |

### Path Validation

| Context | Rules |
|---|---|
| **API spec paths** | Must be relative, no `..` traversal, no absolute paths |
| **CLI template output** | Blocks system directories (`/etc`, `/proc`, `/sys`, `/dev`, `/boot`, `/sbin`) |
| **Backup names** | No `/`, `\`, `..`, or null bytes |

### Metal3 Annotations

Metal3 deployments **fail fast** if required annotations are missing:

| Annotation | Required | Purpose |
|---|---|---|
| `aether.io/boot-mac-address` | Yes | Hardware MAC address for provisioning |
| `aether.io/image-url` | Yes | Bootable disk image URL |
| `aether.io/boot-mode` | No (default: `UEFI`) | Boot mode (`UEFI` or `BIOS`) |
| `aether.io/image-checksum-url` | No | Image integrity verification URL |

---

## 📋 Audit Trail Integrity

Every audit event includes a **SHA-256 integrity hash** computed over its fields:

```
hash = SHA-256("aether-audit-integrity-{id}|{timestamp}|{action}|{workload}|{result}|{message}")
```

| Property | Detail |
|---|---|
| **Hash algorithm** | SHA-256 (hex-encoded) |
| **Fields covered** | id, timestamp, action, workload, result, message |
| **Verification** | `AuditLog::verify_event_integrity(event)` returns `true` if hash matches |
| **Legacy events** | Events without hashes are accepted (backward compatible) |
| **Tamper detection** | Modified events produce a hash mismatch |

### Audit Integrity Verification API

The REST API exposes a `GET /api/audit/verify` endpoint that checks every audit event against its stored SHA-256 hash and returns an integrity report:

```bash
curl http://localhost:5090/api/audit/verify
```

```json
{
  "total": 142,
  "verified": 142,
  "tampered": 0,
  "integrity": "ok",
  "tampered_events": []
}
```

If any event has been modified, `integrity` returns `"compromised"` and `tampered_events` lists the affected event IDs.

---

## 📁 File Permission Hardening

Sensitive files are created with **restrictive permissions** on Unix systems:

| File Type | Permission | Octal | Description |
|---|---|---|---|
| **Backup files** | Owner read/write only | `0o600` | State backups may contain workload config |
| **Snapshot files** | Owner read/write only | `0o600` | Pre-deploy snapshots contain full workload state |
| **Secrets store** | Default (from state atomic write) | — | Protected by advisory file locking |

On non-Unix platforms (Windows), default filesystem permissions apply.

---

## ✅ Confirmation Safety

Destructive operations (delete, rollback, etc.) require explicit confirmation:

| Mode | Behavior |
|---|---|
| **Interactive** | Prompts `[y/N]` — defaults to **No** |
| **`--yes` flag** | Auto-confirms all prompts |
| **`--quiet` mode** | Defaults to **No** (safe) — requires `--yes` to auto-confirm |
| **`--json` mode** | Defaults to **No** (safe) — requires `--yes` to auto-confirm |

This prevents accidental destructive operations in non-interactive contexts (scripts, CI pipelines) unless explicitly opted in with `--yes`.

---

## 🛡️ Policy Engine

The policy engine evaluates workload specs against a set of rules before deployment. Two built-in policy sets are provided: **production** (strict) and **development** (relaxed).

### 10 Rule Types

| # | Rule | Production | Development | Description |
|---|---|---|---|---|
| 1 | **Resource limits** | ✅ Required | ⚠️ Warning | CPU, memory, and storage must be specified |
| 2 | **Image registry** | ✅ Required | ❌ Off | Image must come from an approved registry |
| 3 | **Health checks** | ✅ Required | ❌ Off | Liveness/readiness probes must be defined |
| 4 | **Replicas** | ✅ Minimum 2 | ❌ Off | High availability requires multiple replicas |
| 5 | **Labels/tags** | ✅ Required | ❌ Off | `owner` and `project` metadata must be set |
| 6 | **Network policy** | ✅ Required | ❌ Off | Network isolation must be configured |
| 7 | **Secret references** | ✅ Validated | ❌ Off | Referenced secrets must exist |
| 8 | **Resource caps** | ✅ Enforced | ❌ Off | Resources must not exceed maximum thresholds |
| 9 | **Image tag** | ✅ No `:latest` | ❌ Off | Immutable tags required in production |
| 10 | **Namespace** | ✅ Required | ❌ Off | Workload must specify a namespace |

### Policy Sets

**Production** (`--policy production`):
- All 10 rules enforced
- Violations with `Error` severity **block deployment**
- Default policy set

**Development** (`--policy development`):
- Only resource limits checked (as warnings)
- Designed for fast iteration
- Nothing blocks deployment

### CLI Usage

```bash
# Check against production policies (default)
aether policy-check
aether -s ./my-app.yaml policy-check

# Check against development policies
aether policy-check --policy development
```

---

## 🚦 Policy Gate on Deploy

When you run a deploy command (`aether run`, `aether compose up`, `aether deploy`), the policy engine **automatically evaluates** the workload spec before proceeding.

### Gate Behavior

| Scenario | Result |
|---|---|
| All policies pass | ✅ Deployment proceeds normally |
| Any policy fails (Error severity) | ❌ Deployment is **blocked** with violation details |
| `--skip-policy` flag | ⚠️ Policy checks are skipped entirely |
| Development policy set | ⚠️ Warnings only, deployment proceeds |

### Skipping Policy Checks

```bash
# Skip policy gate (use with caution)
aether --skip-policy run --runtime kube

# Also works with compose
aether --skip-policy compose up

# And batch deploy
aether --skip-policy deploy ./specs/
```

### REST API Policy Check

```bash
curl -X POST http://localhost:5090/api/policy/check \
  -H "Content-Type: application/json" \
  -d '{
    "spec": { ... },
    "policy_set": "production"
  }'
```

| Field | Required | Default | Description |
|---|---|---|---|
| `spec` | Yes | -- | Workload spec as JSON |
| `policy_set` | No | `"production"` | `"production"` or `"development"` |

---

## 🔐 State File Locking

aether uses **advisory file locking** (Unix `flock`) to prevent concurrent write corruption when multiple CLI processes or the API server modify state simultaneously.

### How It Works

```
1. Create/open lock file: state.json.lock
2. Acquire exclusive advisory lock (LOCK_EX, blocking)
3. Write state to temporary file
4. Rename temporary file to state file
5. Release lock (file handle dropped)
```

### Lock Properties

| Property | Detail |
|---|---|
| **Lock type** | Exclusive (`LOCK_EX`) -- only one writer at a time |
| **Blocking** | Yes -- concurrent processes **wait** rather than fail immediately |
| **Lock file** | `<state-file>.lock` (e.g., `state.json.lock`) |
| **Release** | Automatic when the file handle is dropped |

### Error Handling

If the lock cannot be acquired:

```
Error: failed to acquire state file lock (is another aether process running?)
```

### Platform Support

| Platform | Behavior |
|---|---|
| Linux / macOS | Full `flock`-based advisory locking via `libc::flock` |
| Windows / Other | No-op (lock always succeeds) |

---

## ⚛️ Atomic State Persistence

State writes use an **atomic write pattern** (write-to-temp + rename) to prevent corruption from crashes or power loss.

### Write Sequence

```
1. Serialize state to pretty-printed JSON
2. Create parent directory if needed (mkdir -p)
3. Acquire advisory lock on state.json.lock
4. Write JSON to state.json.tmp (temporary file in same directory)
5. Rename state.json.tmp → state.json (atomic on POSIX filesystems)
6. Release lock (drop file handle)
```

### Why This Matters

| Failure Scenario | Without Atomic Write | With Atomic Write |
|---|---|---|
| Crash during write | Corrupted/truncated JSON | Previous valid state preserved |
| Power loss mid-write | Partial data on disk | Either old or new state, never partial |
| Concurrent reads | May read partial data | Always reads a complete, valid file |
| Disk full | Half-written data | Temp file write fails, original preserved |

### Files Involved

| File | Purpose |
|---|---|
| `~/.aether/state.json` | Current workload state (the source of truth) |
| `~/.aether/state.json.tmp` | Temporary write target (renamed on success) |
| `~/.aether/state.json.lock` | Advisory lock file (prevents concurrent writes) |

---

## ☸️ Kubernetes Resource Safety

### Timeouts

All Kubernetes API calls are wrapped with a **5-minute timeout** to prevent indefinite hangs when the API server is unresponsive:

| Operation | Timeout | Behavior on Timeout |
|---|---|---|
| Pod create/delete | 5 min | Error with descriptive message |
| Service/Ingress/PVC create | 5 min | Error + cleanup of created resources |
| ConfigMap/Secret create | 5 min | Error + cleanup |
| Pod logs | 5 min | Error returned to caller |

This matches the Podman adapter's 10-minute timeout for container commands.

### Resource Cleanup on Failure

When pod creation fails after supporting resources (ConfigMap, Secret, PVC, Service, Ingress) have already been created, aether **automatically cleans up orphaned resources**:

```
1. Create ConfigMap  ✓ (tracked)
2. Create Secret     ✓ (tracked)
3. Create PVC        ✓ (tracked)
4. Create Service    ✓ (tracked)
5. Create Pod        ✗ FAILED
   → Clean up: Service, PVC, Secret, ConfigMap (best-effort)
```

### Conflict Handling

Resource creation distinguishes **409 Conflict** (resource already exists) from other errors:

| HTTP Status | Behavior |
|---|---|
| `409 Conflict` | Logged as info, deployment continues |
| `403 Forbidden` | Error returned, deployment aborted |
| `500 Server Error` | Error returned, cleanup triggered |
| Network timeout | Error returned, cleanup triggered |

### Namespace Validation

Both workload name and namespace are validated as DNS-1123 labels before any Kubernetes API calls.

---

## 🚦 Rate Limiting

The API server enforces a **200 concurrent request limit** using tower middleware. When the limit is exceeded, additional requests receive a `503 Service Unavailable` response until in-flight requests complete. This protects the server from resource exhaustion during traffic spikes or misbehaving clients.

| Property | Detail |
|---|---|
| **Limit** | 200 concurrent requests |
| **Scope** | All routes (API and dashboard) |
| **Exceeded behavior** | `503 Service Unavailable` |
| **Implementation** | tower concurrency limit middleware |

---

## 📝 Structured JSON Logging

Set the `AETHER_LOG_FORMAT` environment variable to `json` to switch from human-readable log output to structured JSON lines:

```bash
AETHER_LOG_FORMAT=json aether serve
```

Each log entry is a single JSON object with `timestamp`, `level`, `message`, and `target` fields, making it suitable for ingestion by log aggregation systems such as Elasticsearch, Grafana Loki, or Datadog.

| Variable | Value | Effect |
|---|---|---|
| `AETHER_LOG_FORMAT` | `json` | Structured JSON log output |
| `AETHER_LOG_FORMAT` | unset or any other value | Default human-readable output |

---

## 📡 Webhook Delivery Security

aether supports webhook notifications for events. Channels are managed with `aether webhook` commands.

### Security Considerations

| Aspect | Detail |
|---|---|
| **Transport** | Use HTTPS URLs for webhook endpoints |
| **Authentication** | Include authentication tokens in webhook headers or URL parameters |
| **HTTP method** | Configurable: `POST` (default) or `GET` |
| **Severity filter** | Only events at or above the configured severity trigger delivery (`info`, `warning`, `error`, `critical`) |
| **Retry queue** | Failed deliveries are queued for retry |

### Webhook Commands

```bash
# Add a webhook channel
aether webhook add alerts https://hooks.example.com/notify \
  --method POST \
  --severity warning

# List all channels
aether webhook list

# Send a test notification
aether webhook test alerts

# View pending deliveries in the retry queue
aether webhook queue

# Force-retry all queued webhooks now
aether webhook flush

# Remove a channel
aether webhook remove alerts
```

### Severity Levels

| Level | Priority | Example Events |
|---|---|---|
| `info` | Lowest | Workload deployed, backup created |
| `warning` | Medium | Secret rotation due, resource warning |
| `error` | High | Deployment failed, health check failed |
| `critical` | Highest | Circuit breaker tripped, data corruption |

A webhook configured with `--severity warning` will receive `warning`, `error`, and `critical` events, but not `info` events.

---

## 📧 Email SMTP Notifications

Aether supports real SMTP email delivery for event notifications via the `lettre` crate. Configure an email notification channel using `ChannelType::Email`.

### Configuration

Email channels are configured alongside other notification channels (Console, File, Webhook):

| Field | Required | Description |
|---|---|---|
| `smtp_host` | Yes | SMTP server hostname (e.g., `smtp.gmail.com`) |
| `smtp_port` | No | SMTP port (default: 587 for STARTTLS) |
| `smtp_username` | Yes | SMTP authentication username |
| `smtp_password` | Yes | SMTP authentication password |
| `from_address` | Yes | Sender email address |
| `to_addresses` | Yes | Array of recipient email addresses |
| `severity` | No | Minimum severity to trigger email (`info`, `warning`, `error`, `critical`) |

### Security Considerations

| Aspect | Detail |
|---|---|
| **Transport** | Use TLS/STARTTLS for encrypted SMTP connections |
| **Credentials** | Store SMTP credentials in environment variables or secrets, not in config files |
| **Rate limiting** | Email channels respect the per-channel cooldown period to avoid inbox flooding |
| **Severity filter** | Only events at or above the configured severity trigger email delivery |

---

## 🔗 Cross-References

| Document | Relevance |
|---|---|
| [Compose Guide](./compose.md) | Policy gate integration with compose deployments |
| [Health Monitoring](./health-monitoring.md) | Uptime tracking and health records |
| [Plugin System](./plugins.md) | Custom runtime extensions |
| [API Reference](../reference/api/API-Reference.md) | Full REST API documentation |
| [Quick Reference](../quick-reference/QUICK_REFERENCE.md) | Command cheat sheet |
