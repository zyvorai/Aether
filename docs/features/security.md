# Security Guide 🔒

Encryption, policy enforcement, and state protection in Orchestr8.

---

## AES-256-GCM Secrets Encryption

### How It Works

When `ORCHESTR8_SECRET_KEY` is set to a non-default value:

1. **Key derivation:** User key is hashed with SHA-256 to produce a 32-byte AES key
2. **Encryption:** AES-256-GCM with a random 12-byte nonce per value
3. **Storage:** Nonce prepended to ciphertext, base64-encoded
4. **Authentication:** GCM tag provides integrity verification

### Setup

```bash
# Set production encryption key
export ORCHESTR8_SECRET_KEY="my-production-key-at-least-16-chars"

# Create and set a secret (encrypted with AES-256-GCM)
orchestr8 secrets create db-creds --namespace production
orchestr8 secrets set db-creds password "s3cret!"

# Without the key, XOR obfuscation is used (dev-only)
```

### Encryption Methods

| Method | When Used | Security Level |
|--------|-----------|---------------|
| `aes-256` | `ORCHESTR8_SECRET_KEY` is set | Production-grade |
| `obfuscate` | Default dev key | Dev-only (NOT secure) |
| `vault-ref` | External vault reference | Stored externally |

The `SecretValue.method` field tracks which encryption was used, allowing mixed storage.

---

## Policy Engine

### 10 Built-in Rule Types

| Rule | Description |
|------|-------------|
| `MaxCpu` | Maximum CPU cores |
| `MaxMemoryGi` | Maximum memory in GiB |
| `MaxStorageGi` | Maximum storage in GiB |
| `MaxGpu` | Maximum GPU count |
| `RequireHealthProbes` | Health probes must be defined |
| `RequireTls` | TLS required on ingress |
| `RequireOwner` | Owner metadata required |
| `RequireResourceLimits` | CPU/memory must be non-zero |
| `DisallowRuntime` | Block specific runtimes |
| `MinReplicas` | Minimum replica count |

### Policy Gate on Deploy

Every `orchestr8 run`, `compose up`, and `deploy` command evaluates the workload against the configured policy set. Violations with `Error` severity block deployment.

```yaml
# ~/.orchestr8/config.yaml
policy:
  enforceOnDeploy: true
  policySet: production
```

```bash
# Bypass (use with caution)
orchestr8 run --spec app.yaml --skip-policy
```

---

## State File Protection

### Advisory File Locking

`~/.orchestr8/state.json` is protected with Unix `flock()` during writes. This prevents concurrent `orchestr8` processes from corrupting the state file.

### Atomic Writes

State is written to a temporary file first, then atomically renamed. This ensures the state file is never left in a half-written state, even if the process crashes mid-write.

---

## Cascading Deletion

When a workload is deleted, all associated data is cleaned up:
- Orchestrator registration
- Dependency graph entries
- Scheduler placements
- Health history records

This prevents data accumulation from deleted workloads.

---

## Webhook Security

Webhook payloads are delivered over HTTPS with:
- 10-second timeout per request
- Persistent retry queue (up to 5 attempts)
- Exponential backoff (30s → 30min)
