//! Secrets management engine
//!
//! Encrypt, decrypt, and manage sensitive configuration values.
//! Supports sealed secrets pattern with key rotation and access auditing.

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

/// Rotation policy for secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    /// Rotate every N days
    pub interval_days: u32,
    /// Maximum age before forced rotation
    pub max_age_days: u32,
    /// Notify N days before expiry
    pub notify_before_days: u32,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            interval_days: 90,
            max_age_days: 365,
            notify_before_days: 14,
        }
    }
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
        let encrypted = self.encrypt(value);
        let now = crate::resources::now_rfc3339();

        let secret = self
            .secrets
            .get_mut(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        let version = secret
            .data
            .get(key)
            .map(|v| v.version + 1)
            .unwrap_or(1);

        secret.data.insert(
            key.to_string(),
            SecretValue {
                encrypted,
                method: EncryptionMethod::Obfuscate,
                version,
                last_rotated: now.clone(),
            },
        );
        secret.updated_at = now.clone();
        secret.access_log.push(SecretAccess {
            timestamp: now,
            action: SecretAction::Write,
            key: key.to_string(),
            actor: "cli".to_string(),
        });

        Ok(())
    }

    /// Get a decrypted value from a secret
    pub fn get(&mut self, secret_name: &str, key: &str) -> anyhow::Result<String> {
        let now = crate::resources::now_rfc3339();
        let secret = self
            .secrets
            .get_mut(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        let encrypted = secret
            .data
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Key '{}' not found in secret '{}'", key, secret_name))?
            .encrypted
            .clone();

        secret.access_log.push(SecretAccess {
            timestamp: now,
            action: SecretAction::Read,
            key: key.to_string(),
            actor: "cli".to_string(),
        });

        self.decrypt(&encrypted)
    }

    /// Delete a key from a secret
    pub fn delete_key(&mut self, secret_name: &str, key: &str) -> anyhow::Result<()> {
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
            actor: "cli".to_string(),
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
        let encrypted = self.encrypt(new_value);
        let now = crate::resources::now_rfc3339();

        let secret = self
            .secrets
            .get_mut(secret_name)
            .ok_or_else(|| anyhow::anyhow!("Secret '{}' not found", secret_name))?;

        let version = secret
            .data
            .get(key)
            .map(|v| v.version + 1)
            .unwrap_or(1);

        secret.data.insert(
            key.to_string(),
            SecretValue {
                encrypted,
                method: EncryptionMethod::Obfuscate,
                version,
                last_rotated: now.clone(),
            },
        );
        secret.updated_at = now.clone();
        secret.access_log.push(SecretAccess {
            timestamp: now,
            action: SecretAction::Rotate,
            key: key.to_string(),
            actor: "cli".to_string(),
        });

        Ok(())
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
        crate::resources::orchestr8_path("secrets.json")
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
        // In production, this would come from environment variable or HSM
        // This is a development-only key
        b"orchestr8-dev-key-do-not-use-prod".to_vec()
    }

    fn encrypt(&self, plaintext: &str) -> String {
        // Simple XOR obfuscation for development
        // In production, use ring/aes-gcm for AES-256-GCM
        let key = &self.encryption_key;
        let encrypted: Vec<u8> = plaintext
            .as_bytes()
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        base64_encode(&encrypted)
    }

    fn decrypt(&self, ciphertext: &str) -> anyhow::Result<String> {
        let encrypted = base64_decode(ciphertext)?;
        let key = &self.encryption_key;
        let decrypted: Vec<u8> = encrypted
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        String::from_utf8(decrypted).map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    fn needs_rotation(&self, secret: &Secret) -> bool {
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
            (now - dt.with_timezone(&chrono::Utc))
                .num_seconds() as f64
                / 86400.0
        } else {
            0.0
        }
    }
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
    let mut output = String::new();
    output.push_str("Secrets:\n\n");

    if summaries.is_empty() {
        output.push_str("  No secrets stored.\n");
        return output;
    }

    for s in summaries {
        output.push_str(&format!(
            "  {} ({}) - {} keys\n",
            s.name, s.namespace, s.key_count
        ));
        output.push_str(&format!("    Updated: {}\n", &s.updated_at[..19]));
        if s.needs_rotation {
            output.push_str("    ⚠ Rotation needed\n");
        }
        output.push('\n');
    }

    output
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
    fn test_encrypt_decrypt_roundtrip() {
        let store = SecretStore::new();
        let plaintext = "hello-world-secret-value";
        let encrypted = store.encrypt(plaintext);
        let decrypted = store.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
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
