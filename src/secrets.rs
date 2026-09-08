// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Secrets management engine
//!
//! Encrypt, decrypt, and manage sensitive configuration values.
//! Supports sealed secrets pattern with key rotation and access auditing.

use crate::output;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A managed secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub name: String,
    pub namespace: String,
    pub data: HashMap<String, SecretValue>,
    pub created_at: String,
    pub updated_at: String,
    pub rotation_policy: Option<RotationPolicy>,
    pub access_log: Vec<SecretAccess>,
}

/// A secret value (stored encrypted at rest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretValue {
    /// Base64-encoded encrypted value
    pub encrypted: String,
    /// Encryption method used
    pub method: EncryptionMethod,
    /// Version for rotation tracking
    pub version: u32,
    /// When this value was last rotated
    pub last_rotated: String,
}

/// Encryption method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionMethod {
    /// XOR-based obfuscation (for local dev only)
    Obfuscate,
    /// AES-256 encryption (production)
    Aes256,
    /// External vault reference (not stored locally)
    VaultRef,
}

impl std::fmt::Display for EncryptionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionMethod::Obfuscate => write!(f, "obfuscate"),
            EncryptionMethod::Aes256 => write!(f, "aes-256"),
            EncryptionMethod::VaultRef => write!(f, "vault-ref"),
        }
    }
}

/// Attestation-gated secret reference (released only after Ragnarok verify).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestGatedSecret {
    pub name: String,
    pub vm_id: String,
    pub provider: String,
    pub release_token: Option<String>,
    pub attestation_required: bool,
}

impl AttestGatedSecret {
    pub fn from_workload_spec(spec: &crate::spec::Workload, secret_name: &str) -> Option<Self> {
        let conf = spec.confidential.as_ref()?;
        if !conf.enabled
            || conf.secrets.release_policy != crate::spec::SecretReleasePolicy::AttestGated
        {
            return None;
        }
        Some(Self {
            name: secret_name.to_string(),
            vm_id: spec.metadata.name.clone(),
            provider: format!("{:?}", conf.secrets.provider).to_lowercase(),
            release_token: None,
            attestation_required: conf.attestation.required,
        })
    }
}

/// Rotation policy for secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    /// Rotate every N days
    pub interval_days: u32,
    /// Maximum age before forced rotation
    pub max_age_days: u32,
    /// Notify N days before expiry
    pub notify_before_days: u32,
    /// Whether Aether owns this value and may generate a fresh random
    /// credential on rotation. When `false` (the default) the value mirrors an
    /// externally-managed credential and MUST NOT be auto-generated — automated
    /// rotation only alerts and defers to an operator/external system.
    #[serde(default)]
    pub generate: bool,
}

impl RotationPolicy {
    /// Validate that the policy fields are consistent.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.interval_days == 0 {
            anyhow::bail!("rotation interval_days must be > 0");
        }
        if self.max_age_days < self.interval_days {
            anyhow::bail!(
                "max_age_days ({}) must be >= interval_days ({})",
                self.max_age_days,
                self.interval_days
            );
        }
        if self.notify_before_days > self.max_age_days {
            anyhow::bail!(
                "notify_before_days ({}) must be <= max_age_days ({})",
                self.notify_before_days,
                self.max_age_days
            );
        }
        Ok(())
    }
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            interval_days: 90,
            max_age_days: 365,
            notify_before_days: 14,
            generate: false,
        }
    }
}

/// Generate a cryptographically-random alphanumeric credential of `len`
/// characters, drawn from the OS CSPRNG. Used for auto-rotating secrets whose
/// value Aether owns (`RotationPolicy.generate == true`).
pub fn generate_secret_value(len: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::rngs::OsRng;
    use rand::Rng;
    OsRng
        .sample_iter(&Alphanumeric)
        .take(len.max(1))
        .map(char::from)
        .collect()
}

/// Secret access log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretAccess {
    pub timestamp: String,
    pub action: SecretAction,
    pub key: String,
    pub actor: String,
}

/// Type of secret access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecretAction {
    Read,
    Write,
    Delete,
    Rotate,
}

impl std::fmt::Display for SecretAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretAction::Read => write!(f, "READ"),
            SecretAction::Write => write!(f, "WRITE"),
            SecretAction::Delete => write!(f, "DELETE"),
            SecretAction::Rotate => write!(f, "ROTATE"),
        }
    }
}

/// Secrets manager
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretStore {
    secrets: HashMap<String, Secret>,
    /// Encryption key (in production, loaded from environment/HSM)
    #[serde(skip)]
    encryption_key: Vec<u8>,
}

impl SecretStore {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
            encryption_key: Self::default_key(),
        }
    }

    /// Create with a specific encryption key
    pub fn with_key(key: Vec<u8>) -> Self {
        Self {
            secrets: HashMap::new(),
            encryption_key: key,
        }
    }

    /// Create a new secret
    pub fn create_secret(&mut self, name: &str, namespace: &str) -> &mut Secret {
        let now = crate::resources::now_rfc3339();
        let secret = Secret {
            name: name.to_string(),
            namespace: namespace.to_string(),
            data: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
            rotation_policy: Some(RotationPolicy::default()),
            access_log: Vec::new(),
        };
        self.secrets.insert(name.to_string(), secret);
        self.secrets.get_mut(name).expect("just inserted")
    }

    /// Set a key-value pair in a secret
    pub fn set(&mut self, secret_name: &str, key: &str, value: &str) -> anyhow::Result<()> {
        self.set_with_actor(secret_name, key, value, "cli")
    }

    /// Set a key-value pair in a secret with a specific actor for audit logging
    pub fn set_with_actor(
        &mut self,
        secret_name: &str,
        key: &str,
        value: &str,
        actor: &str,
    ) -> anyhow::Result<()> {
        let (encrypted, method) = self.encrypt(value)?;
        let now = crate::resources::now_rfc3339();

        let secret = self
            .secrets
            .get_mut(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        let version = secret.data.get(key).map(|v| v.version + 1).unwrap_or(1);

        secret.data.insert(
            key.to_string(),
            SecretValue {
                encrypted,
                method,
                version,
                last_rotated: now.clone(),
            },
        );
        secret.updated_at = now.clone();
        secret.access_log.push(SecretAccess {
            timestamp: now,
            action: SecretAction::Write,
            key: key.to_string(),
            actor: actor.to_string(),
        });

        Ok(())
    }

    /// Get a decrypted value from a secret
    pub fn get(&self, secret_name: &str, key: &str) -> anyhow::Result<String> {
        let secret = self
            .secrets
            .get(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        let sv = secret.data.get(key).ok_or_else(|| {
            anyhow::anyhow!("Key '{}' not found in secret '{}'", key, secret_name)
        })?;

        self.decrypt(&sv.encrypted, &sv.method)
    }

    /// Get a decrypted value and log the access
    pub fn get_and_log(&mut self, secret_name: &str, key: &str) -> anyhow::Result<String> {
        self.get_and_log_with_actor(secret_name, key, "cli")
    }

    /// Get a decrypted value and log the access with a specific actor
    pub fn get_and_log_with_actor(
        &mut self,
        secret_name: &str,
        key: &str,
        actor: &str,
    ) -> anyhow::Result<String> {
        let now = crate::resources::now_rfc3339();
        let secret = self
            .secrets
            .get_mut(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        let sv = secret.data.get(key).ok_or_else(|| {
            anyhow::anyhow!("Key '{}' not found in secret '{}'", key, secret_name)
        })?;
        let encrypted = sv.encrypted.clone();
        let method = sv.method.clone();

        secret.access_log.push(SecretAccess {
            timestamp: now,
            action: SecretAction::Read,
            key: key.to_string(),
            actor: actor.to_string(),
        });

        self.decrypt(&encrypted, &method)
    }

    /// Delete a key from a secret
    pub fn delete_key(&mut self, secret_name: &str, key: &str) -> anyhow::Result<()> {
        self.delete_key_with_actor(secret_name, key, "cli")
    }

    /// Delete a key from a secret with a specific actor for audit logging
    pub fn delete_key_with_actor(
        &mut self,
        secret_name: &str,
        key: &str,
        actor: &str,
    ) -> anyhow::Result<()> {
        let now = crate::resources::now_rfc3339();
        let secret = self
            .secrets
            .get_mut(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        secret.data.remove(key);
        secret.updated_at = now.clone();
        secret.access_log.push(SecretAccess {
            timestamp: now,
            action: SecretAction::Delete,
            key: key.to_string(),
            actor: actor.to_string(),
        });

        Ok(())
    }

    /// Delete an entire secret
    pub fn delete_secret(&mut self, name: &str) -> Option<Secret> {
        self.secrets.remove(name)
    }

    /// List all secrets (without values)
    pub fn list(&self) -> Vec<SecretSummary> {
        self.secrets
            .values()
            .map(|s| SecretSummary {
                name: s.name.clone(),
                namespace: s.namespace.clone(),
                key_count: s.data.len(),
                created_at: s.created_at.clone(),
                updated_at: s.updated_at.clone(),
                needs_rotation: self.needs_rotation(s),
            })
            .collect()
    }

    /// Get a secret's metadata
    pub fn get_secret(&self, name: &str) -> Option<&Secret> {
        self.secrets.get(name)
    }

    /// Rotate a specific key in a secret
    pub fn rotate(&mut self, secret_name: &str, key: &str, new_value: &str) -> anyhow::Result<()> {
        self.rotate_with_actor(secret_name, key, new_value, "cli")
    }

    /// Rotate a specific key in a secret with a specific actor for audit logging
    pub fn rotate_with_actor(
        &mut self,
        secret_name: &str,
        key: &str,
        new_value: &str,
        actor: &str,
    ) -> anyhow::Result<()> {
        let (encrypted, method) = self.encrypt(new_value)?;
        let now = crate::resources::now_rfc3339();

        let secret = self
            .secrets
            .get_mut(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        let version = secret.data.get(key).map(|v| v.version + 1).unwrap_or(1);

        secret.data.insert(
            key.to_string(),
            SecretValue {
                encrypted,
                method,
                version,
                last_rotated: now.clone(),
            },
        );
        secret.updated_at = now.clone();
        secret.access_log.push(SecretAccess {
            timestamp: now,
            action: SecretAction::Rotate,
            key: key.to_string(),
            actor: actor.to_string(),
        });

        Ok(())
    }

    /// Rotate a key to a freshly-generated random credential. Only valid for
    /// secrets Aether owns (`rotation_policy.generate == true`); callers should
    /// gate on [`SecretStore::auto_generatable`] first. The generated plaintext
    /// is stored encrypted and never returned or logged — only the new version
    /// number is surfaced. Returns an error for external/mirrored secrets so a
    /// caller can never accidentally overwrite a value Aether does not own.
    pub fn rotate_generated(
        &mut self,
        secret_name: &str,
        key: &str,
        actor: &str,
    ) -> anyhow::Result<u32> {
        if !self.auto_generatable(secret_name) {
            anyhow::bail!(
                "Secret '{}' is not auto-generatable (rotation_policy.generate=false); \
                 it mirrors an externally-managed credential and must be rotated externally",
                secret_name
            );
        }
        let value = generate_secret_value(32);
        self.rotate_with_actor(secret_name, key, &value, actor)?;
        // Value dropped here; recover the version we just wrote for the caller.
        let version = self
            .get_secret(secret_name)
            .and_then(|s| s.data.get(key))
            .map(|v| v.version)
            .unwrap_or(0);
        Ok(version)
    }

    /// Whether a secret's value is Aether-owned and safe to auto-generate on
    /// rotation. False when there is no rotation policy or `generate` is unset.
    pub fn auto_generatable(&self, secret_name: &str) -> bool {
        self.get_secret(secret_name)
            .and_then(|s| s.rotation_policy.as_ref())
            .map(|p| p.generate)
            .unwrap_or(false)
    }

    /// Test helper: backdate a key's `last_rotated` so rotation audits treat it
    /// as due/overdue without waiting real time.
    #[cfg(test)]
    pub(crate) fn set_last_rotated_days_ago(&mut self, secret_name: &str, key: &str, days: i64) {
        let ts = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        if let Some(v) = self
            .secrets
            .get_mut(secret_name)
            .and_then(|s| s.data.get_mut(key))
        {
            v.last_rotated = ts;
        }
    }

    /// Check which secrets need rotation
    pub fn audit_rotation(&self) -> Vec<RotationAlert> {
        let mut alerts = Vec::new();

        for secret in self.secrets.values() {
            if let Some(policy) = &secret.rotation_policy {
                for (key, value) in &secret.data {
                    let age_days = self.age_in_days(&value.last_rotated);
                    if age_days > policy.max_age_days as f64 {
                        alerts.push(RotationAlert {
                            secret: secret.name.clone(),
                            key: key.clone(),
                            age_days: age_days as u32,
                            max_age_days: policy.max_age_days,
                            severity: AlertSeverity::Critical,
                            message: format!(
                                "Key '{}' in '{}' is {} days old (max: {})",
                                key, secret.name, age_days as u32, policy.max_age_days
                            ),
                        });
                    } else if age_days > (policy.max_age_days - policy.notify_before_days) as f64 {
                        alerts.push(RotationAlert {
                            secret: secret.name.clone(),
                            key: key.clone(),
                            age_days: age_days as u32,
                            max_age_days: policy.max_age_days,
                            severity: AlertSeverity::Warning,
                            message: format!(
                                "Key '{}' in '{}' expires in {} days",
                                key,
                                secret.name,
                                policy.max_age_days as f64 - age_days
                            ),
                        });
                    }
                }
            }
        }

        alerts
    }

    /// Default path for secrets store
    pub fn default_path() -> PathBuf {
        crate::resources::aether_path("secrets.json")
    }

    /// Load from disk
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let mut store: Self = crate::resources::json_load(path)?;
        store.encryption_key = Self::default_key();
        Ok(store)
    }

    /// Save to disk
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        crate::resources::json_save(self, path)
    }

    // --- Private ---

    fn default_key() -> Vec<u8> {
        match std::env::var("AETHER_SECRET_KEY") {
            Ok(key) if key.len() >= 16 => key.into_bytes(),
            Ok(key) if !key.is_empty() => {
                tracing::warn!(
                    "AETHER_SECRET_KEY is too short ({} bytes, minimum 16). \
                     Using it anyway, but consider a longer key for production.",
                    key.len()
                );
                key.into_bytes()
            }
            _ => {
                tracing::warn!(
                    "AETHER_SECRET_KEY not set — generating deterministic key from machine identity. \
                     Set AETHER_SECRET_KEY for stable production encryption."
                );
                // Derive a machine-specific key from hostname + user rather than
                // using a hardcoded constant that any source reader can extract.
                // NOTE: process::id() is intentionally excluded — it changes on
                // every restart, which would make secrets unrecoverable.
                use sha2::{Digest, Sha256};
                let hostname = std::env::var("HOSTNAME")
                    .or_else(|_| std::env::var("USER"))
                    .unwrap_or_else(|_| "aether-local".to_string());
                let seed = format!("aether-machine-key-{}", hostname);
                Sha256::digest(seed.as_bytes()).to_vec()
            }
        }
    }

    /// Encrypt a plaintext value. Always uses AES-256-GCM regardless of key source.
    fn encrypt(&self, plaintext: &str) -> anyhow::Result<(String, EncryptionMethod)> {
        Ok((self.aes_encrypt(plaintext)?, EncryptionMethod::Aes256))
    }

    /// Decrypt a ciphertext value, dispatching by encryption method.
    fn decrypt(&self, ciphertext: &str, method: &EncryptionMethod) -> anyhow::Result<String> {
        match method {
            EncryptionMethod::Aes256 => self.aes_decrypt(ciphertext),
            EncryptionMethod::Obfuscate => {
                anyhow::bail!(
                    "XOR-obfuscated secrets are no longer supported. \
                     Re-encrypt with `aether secrets set` to upgrade to AES-256-GCM."
                )
            }
            EncryptionMethod::VaultRef => {
                anyhow::bail!(
                    "VaultRef secrets are stored externally and cannot be decrypted locally"
                )
            }
        }
    }

    fn aes_encrypt(&self, plaintext: &str) -> anyhow::Result<String> {
        use aes_gcm::aead::rand_core::RngCore;
        use aes_gcm::aead::{Aead, KeyInit, OsRng};
        use aes_gcm::{Aes256Gcm, Nonce};
        use sha2::{Digest, Sha256};

        // Derive 32-byte key from user key via SHA-256
        let key_bytes: [u8; 32] = Sha256::digest(&self.encryption_key).into();
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to initialize AES-256-GCM cipher: {}", e))?;

        // Generate random 12-byte nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("AES-256-GCM encryption failed: {}", e))?;

        // Prepend nonce to ciphertext, then base64 encode
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(base64_encode(&combined))
    }

    fn aes_decrypt(&self, ciphertext: &str) -> anyhow::Result<String> {
        use aes_gcm::aead::{Aead, KeyInit};
        use aes_gcm::{Aes256Gcm, Nonce};
        use sha2::{Digest, Sha256};

        let combined = base64_decode(ciphertext)?;
        if combined.len() < 12 {
            anyhow::bail!("Invalid AES ciphertext: too short (missing nonce)");
        }

        let (nonce_bytes, encrypted) = combined.split_at(12);
        let key_bytes: [u8; 32] = Sha256::digest(&self.encryption_key).into();
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("32-byte key");
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = cipher.decrypt(nonce, encrypted).map_err(|_| {
            anyhow::anyhow!("AES-256-GCM decryption failed (wrong key or corrupted data)")
        })?;

        String::from_utf8(plaintext)
            .map_err(|e| anyhow::anyhow!("Decrypted data is not valid UTF-8: {}", e))
    }

    pub fn needs_rotation(&self, secret: &Secret) -> bool {
        if let Some(policy) = &secret.rotation_policy {
            for value in secret.data.values() {
                if self.age_in_days(&value.last_rotated) > policy.interval_days as f64 {
                    return true;
                }
            }
        }
        false
    }

    fn age_in_days(&self, timestamp: &str) -> f64 {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
            let now = chrono::Utc::now();
            (now - dt.with_timezone(&chrono::Utc)).num_seconds() as f64 / 86400.0
        } else {
            0.0
        }
    }
}

/// Encrypt an arbitrary UTF-8 blob with AES-256-GCM using the ambient key
/// (`AETHER_SECRET_KEY`, or a machine-derived key when unset). Returns
/// `base64(nonce || ciphertext)`. Shared with the backup subsystem so backups
/// can be encrypted at rest with the same scheme as secrets.
pub fn encrypt_blob(plaintext: &str) -> anyhow::Result<String> {
    use aes_gcm::aead::rand_core::RngCore;
    use aes_gcm::aead::{Aead, KeyInit, OsRng};
    use aes_gcm::{Aes256Gcm, Nonce};
    use sha2::{Digest, Sha256};

    let key = SecretStore::default_key();
    let key_bytes: [u8; 32] = Sha256::digest(&key).into();
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to initialize AES-256-GCM cipher: {}", e))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("AES-256-GCM encryption failed: {}", e))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(base64_encode(&combined))
}

/// Decrypt a blob produced by [`encrypt_blob`].
pub fn decrypt_blob(ciphertext: &str) -> anyhow::Result<String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    use sha2::{Digest, Sha256};

    let key = SecretStore::default_key();
    let combined = base64_decode(ciphertext)?;
    if combined.len() < 12 {
        anyhow::bail!("Invalid AES ciphertext: too short (missing nonce)");
    }

    let (nonce_bytes, encrypted) = combined.split_at(12);
    let key_bytes: [u8; 32] = Sha256::digest(&key).into();
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).expect("32-byte key");
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, encrypted).map_err(|_| {
        anyhow::anyhow!("AES-256-GCM decryption failed (wrong key or corrupted data)")
    })?;
    String::from_utf8(plaintext)
        .map_err(|e| anyhow::anyhow!("Decrypted data is not valid UTF-8: {}", e))
}

// Simple base64 encode/decode without external dependency
fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    let _ = write!(result, ""); // suppress unused warning
    result
}

fn base64_decode(input: &str) -> anyhow::Result<Vec<u8>> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();

    let decode_char = |c: char| -> anyhow::Result<u32> {
        match c {
            'A'..='Z' => Ok(c as u32 - 'A' as u32),
            'a'..='z' => Ok(c as u32 - 'a' as u32 + 26),
            '0'..='9' => Ok(c as u32 - '0' as u32 + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            _ => Err(anyhow::anyhow!("Invalid base64 character: {}", c)),
        }
    };

    let chars: Vec<char> = input.chars().collect();
    for chunk in chars.chunks(4) {
        let mut triple = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            triple |= decode_char(c)? << (18 - i * 6);
        }
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk.len() > 2 {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk.len() > 3 {
            result.push((triple & 0xFF) as u8);
        }
    }

    Ok(result)
}

/// Summary of a secret (no values exposed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSummary {
    pub name: String,
    pub namespace: String,
    pub key_count: usize,
    pub created_at: String,
    pub updated_at: String,
    pub needs_rotation: bool,
}

/// Rotation alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationAlert {
    pub secret: String,
    pub key: String,
    pub age_days: u32,
    pub max_age_days: u32,
    pub severity: AlertSeverity,
    pub message: String,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Format secrets list
pub fn format_secrets_list(summaries: &[SecretSummary]) -> String {
    let mut out = String::new();

    if summaries.is_empty() {
        out.push_str("  No secrets stored.\n");
        return out;
    }

    let rows: Vec<Vec<String>> = summaries
        .iter()
        .map(|s| {
            vec![
                s.name.clone(),
                s.namespace.clone(),
                s.key_count.to_string(),
                s.updated_at.get(..19).unwrap_or(&s.updated_at).to_string(),
                if s.needs_rotation { "⚠ Yes" } else { "OK" }.to_string(),
            ]
        })
        .collect();
    out.push_str(&format!(
        "\n{}\n",
        output::table(&["Name", "Namespace", "Keys", "Updated", "Rotation"], rows),
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_secret() {
        let mut store = SecretStore::new();
        store.create_secret("db-creds", "production");
        store.set("db-creds", "password", "s3cret123").unwrap();

        let value = store.get("db-creds", "password").unwrap();
        assert_eq!(value, "s3cret123");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_default_key() {
        let store = SecretStore::new();
        let plaintext = "hello-world-secret-value";
        let (encrypted, method) = store.encrypt(plaintext).unwrap();
        assert_eq!(method, EncryptionMethod::Aes256);
        let decrypted = store.decrypt(&encrypted, &method).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_xor_obfuscate_decrypt_rejected() {
        let store = SecretStore::new();
        let result = store.decrypt("dGVzdA==", &EncryptionMethod::Obfuscate);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no longer supported"));
    }

    #[test]
    fn test_rotation_policy_validation() {
        let mut policy = RotationPolicy::default();
        assert!(policy.validate().is_ok());

        policy.interval_days = 0;
        assert!(policy.validate().is_err());

        policy.interval_days = 90;
        policy.max_age_days = 30; // less than interval
        assert!(policy.validate().is_err());

        policy.max_age_days = 365;
        policy.notify_before_days = 400; // more than max_age
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_generate_secret_value_is_random_and_sized() {
        let a = generate_secret_value(32);
        let b = generate_secret_value(32);
        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 32);
        assert_ne!(a, b, "two generated values must differ");
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
        // Never the old placeholder shape.
        assert!(!a.starts_with("rotated-"));
        // len 0 is clamped to at least 1 char (no panic / empty credential).
        assert_eq!(generate_secret_value(0).len(), 1);
    }

    #[test]
    fn test_auto_generatable_reflects_policy() {
        let mut store = SecretStore::new();
        // Default policy has generate=false → not auto-generatable.
        store.create_secret("ext-cred", "prod");
        assert!(!store.auto_generatable("ext-cred"));
        assert!(!store.auto_generatable("missing"));

        // Mark a secret as Aether-owned.
        let owned = store.create_secret("owned-cred", "prod");
        owned.rotation_policy = Some(RotationPolicy {
            generate: true,
            ..RotationPolicy::default()
        });
        assert!(store.auto_generatable("owned-cred"));
    }

    #[test]
    fn test_rotate_generated_only_for_owned_secrets() {
        let mut store = SecretStore::new();
        store.create_secret("ext-cred", "prod");
        store.set("ext-cred", "token", "external-value").unwrap();

        // External secret: refuse to overwrite, value stays intact.
        let err = store
            .rotate_generated("ext-cred", "token", "test")
            .unwrap_err();
        assert!(err.to_string().contains("not auto-generatable"));
        assert_eq!(store.get("ext-cred", "token").unwrap(), "external-value");

        // Owned secret: rotate to a fresh random credential.
        let owned = store.create_secret("owned-cred", "prod");
        owned.rotation_policy = Some(RotationPolicy {
            generate: true,
            ..RotationPolicy::default()
        });
        store.set("owned-cred", "password", "initial").unwrap();
        let version = store
            .rotate_generated("owned-cred", "password", "test")
            .unwrap();
        assert_eq!(version, 2, "version bumps on rotation");
        let rotated = store.get("owned-cred", "password").unwrap();
        assert_ne!(rotated, "initial");
        assert!(!rotated.starts_with("rotated-"), "no placeholder value");
        assert_eq!(rotated.len(), 32);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_aes() {
        let store = SecretStore::with_key(b"my-production-secret-key-32chars!".to_vec());
        let plaintext = "super-secret-database-password";
        let (encrypted, method) = store.encrypt(plaintext).unwrap();
        assert_eq!(method, EncryptionMethod::Aes256);
        let decrypted = store.decrypt(&encrypted, &method).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_different_nonces() {
        let store = SecretStore::with_key(b"test-key-for-nonce-check".to_vec());
        let plaintext = "same-value";
        let (enc1, _) = store.encrypt(plaintext).unwrap();
        let (enc2, _) = store.encrypt(plaintext).unwrap();
        // Different nonces should produce different ciphertexts
        assert_ne!(enc1, enc2);
    }

    #[test]
    fn test_aes_wrong_key_fails() {
        let store1 = SecretStore::with_key(b"key-one-for-encryption".to_vec());
        let store2 = SecretStore::with_key(b"key-two-different-key".to_vec());
        let (encrypted, _) = store1.encrypt("secret-data").unwrap();
        let result = store2.aes_decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_ref_decrypt_fails() {
        let store = SecretStore::new();
        let result = store.decrypt("anything", &EncryptionMethod::VaultRef);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("VaultRef"));
    }

    #[test]
    fn test_delete_key() {
        let mut store = SecretStore::new();
        store.create_secret("app", "default");
        store.set("app", "key1", "val1").unwrap();
        store.set("app", "key2", "val2").unwrap();

        store.delete_key("app", "key1").unwrap();

        assert!(store.get("app", "key1").is_err());
        assert_eq!(store.get("app", "key2").unwrap(), "val2");
    }

    #[test]
    fn test_secret_versioning() {
        let mut store = SecretStore::new();
        store.create_secret("app", "default");
        store.set("app", "token", "v1").unwrap();
        store.set("app", "token", "v2").unwrap();

        let secret = store.get_secret("app").unwrap();
        assert_eq!(secret.data["token"].version, 2);
    }

    #[test]
    fn test_rotation() {
        let mut store = SecretStore::new();
        store.create_secret("app", "default");
        store.set("app", "token", "old-value").unwrap();
        store.rotate("app", "token", "new-value").unwrap();

        let value = store.get("app", "token").unwrap();
        assert_eq!(value, "new-value");

        let secret = store.get_secret("app").unwrap();
        assert!(secret
            .access_log
            .iter()
            .any(|a| a.action == SecretAction::Rotate));
    }

    #[test]
    fn test_list_secrets() {
        let mut store = SecretStore::new();
        store.create_secret("db-creds", "production");
        store.set("db-creds", "password", "secret").unwrap();
        store.create_secret("api-keys", "staging");
        store.set("api-keys", "key1", "abc").unwrap();
        store.set("api-keys", "key2", "def").unwrap();

        let list = store.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_format_list() {
        let mut store = SecretStore::new();
        store.create_secret("test", "default");
        store.set("test", "key", "val").unwrap();
        let output = format_secrets_list(&store.list());
        assert!(output.contains("test"));
    }
}
