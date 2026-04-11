# 🔐 Security Features

> Encryption, secrets management, policy enforcement, and safe state persistence.

---

## 📑 Table of Contents

- [AES-256-GCM Encryption](#-aes-256-gcm-encryption)
- [Setting the Encryption Key](#-setting-the-encryption-key)
- [Secret Lifecycle](#-secret-lifecycle)
- [Policy Engine](#-policy-engine)
- [Policy Gate on Deploy](#-policy-gate-on-deploy)
- [State File Locking](#-state-file-locking)
- [Atomic State Persistence](#-atomic-state-persistence)
- [Webhook Delivery Security](#-webhook-delivery-security)
- [Cross-References](#-cross-references)

---

## 🔒 AES-256-GCM Encryption

orchestr8 encrypts all secret values at rest using **AES-256-GCM** (Galois/Counter Mode), a widely trusted authenticated encryption algorithm.

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

When `ORCHESTR8_SECRET_KEY` is **not set**, orchestr8 falls back to **XOR obfuscation** with a built-in dev key. This is **NOT secure** and is designed only for local development convenience.

```
⚠️  Secret stored with XOR obfuscation (dev-only).
    Set ORCHESTR8_SECRET_KEY for AES-256 encryption.
```

### Encryption Methods

| Method | When Used | Security Level | Icon |
|---|---|---|---|
| `Aes256` | Production key set via `ORCHESTR8_SECRET_KEY` | 🟢 Production-grade | 🔒 |
| `Obfuscate` | No key set (built-in dev key) | 🔴 Dev-only, NOT secure | ⚠️ |
| `VaultRef` | External vault reference (not stored locally) | 🟢 Delegated to vault | 🏛️ |

The `SecretValue.method` field tracks which encryption was used per key, allowing mixed-method storage within a single secret.

---

## 🔑 Setting the Encryption Key

Set the `ORCHESTR8_SECRET_KEY` environment variable to enable AES-256-GCM encryption.

```bash
# Set for the current session
export ORCHESTR8_SECRET_KEY="my-production-secret-key-at-least-16-chars"

# Or in your shell profile (~/.bashrc, ~/.zshrc)
echo 'export ORCHESTR8_SECRET_KEY="your-key-here"' >> ~/.bashrc

# Or per-command
ORCHESTR8_SECRET_KEY="key" orchestr8 secrets set db-creds password "s3cret"
```

### Key Requirements

| Requirement | Detail |
|---|---|
| **Length** | Any length works (hashed via SHA-256 to 32 bytes). At least 16 characters recommended. |
| **Consistency** | Same key must be used for both encryption and decryption |
| **Rotation** | Changing the key invalidates all previously encrypted values |
| **Empty string** | Falls back to the built-in dev key |
| **Character set** | Any UTF-8 bytes are accepted |

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
orchestr8 secrets create db-credentials --namespace production
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
orchestr8 secrets set db-credentials password "s3cret123"
orchestr8 secrets set db-credentials username "admin"
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
orchestr8 secrets get db-credentials password
# Output: s3cret123
```

Each `get` operation:

1. Looks up the secret and key
2. Decrypts using the method recorded in `SecretValue.method`
3. Logs a `READ` event to the access audit trail

---

### Rotate a Secret Value

```bash
orchestr8 secrets set db-credentials password "new-s3cret-456"
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
orchestr8 secrets audit
```

Checks all secrets against their rotation policies and produces alerts:

| Severity | Condition | Message Example |
|---|---|---|
| ⚠️ **Warning** | Key age exceeds `max_age_days - notify_before_days` | `"Key 'password' in 'db-creds' expires in 10 days"` |
| 🔴 **Critical** | Key age exceeds `max_age_days` | `"Key 'password' in 'db-creds' is 400 days old (max: 365)"` |

---

### List All Secrets

```bash
orchestr8 secrets list
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
| `actor` | string | Who performed the action (currently `"cli"`) |

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
orchestr8 policy-check
orchestr8 -s ./my-app.yaml policy-check

# Check against development policies
orchestr8 policy-check --policy development
```

---

## 🚦 Policy Gate on Deploy

When you run a deploy command (`orchestr8 run`, `orchestr8 compose up`, `orchestr8 deploy`), the policy engine **automatically evaluates** the workload spec before proceeding.

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
orchestr8 --skip-policy run --runtime kube

# Also works with compose
orchestr8 --skip-policy compose up

# And batch deploy
orchestr8 --skip-policy deploy ./specs/
```

### REST API Policy Check

```bash
curl -X POST http://localhost:8080/api/policy/check \
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

orchestr8 uses **advisory file locking** (Unix `flock`) to prevent concurrent write corruption when multiple CLI processes or the API server modify state simultaneously.

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
Error: failed to acquire state file lock (is another orchestr8 process running?)
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
| `~/.orchestr8/state.json` | Current workload state (the source of truth) |
| `~/.orchestr8/state.json.tmp` | Temporary write target (renamed on success) |
| `~/.orchestr8/state.json.lock` | Advisory lock file (prevents concurrent writes) |

---

## 📡 Webhook Delivery Security

orchestr8 supports webhook notifications for events. Channels are managed with `orchestr8 webhook` commands.

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
orchestr8 webhook add alerts https://hooks.example.com/notify \
  --method POST \
  --severity warning

# List all channels
orchestr8 webhook list

# Send a test notification
orchestr8 webhook test alerts

# View pending deliveries in the retry queue
orchestr8 webhook queue

# Force-retry all queued webhooks now
orchestr8 webhook flush

# Remove a channel
orchestr8 webhook remove alerts
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

## 🔗 Cross-References

| Document | Relevance |
|---|---|
| [Compose Guide](./compose.md) | Policy gate integration with compose deployments |
| [Health Monitoring](./health-monitoring.md) | Uptime tracking and health records |
| [Plugin System](./plugins.md) | Custom runtime extensions |
| [API Reference](../reference/api/API-Reference.md) | Full REST API documentation |
| [Quick Reference](../quick-reference/QUICK_REFERENCE.md) | Command cheat sheet |
