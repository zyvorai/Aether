// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

//! Zeus OS capacity license verification and enforcement.
//!
//! License unit: managed Kubernetes node count.
//! License file: `.zyvor` — a JSON envelope signed with Zyvor's RSA-2048 key.
//! Enforcement: warning-only in v0.3.0.

pub mod claims;
pub mod enforcement;
pub mod status;
pub mod usage;
pub mod verifier;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use claims::LicenseClaims;
pub use enforcement::{compute_state, state_to_audit_action, LicenseState};
pub use status::{LicenseStatusResponse, LicenseUsageResponse};

/// Default path for the .zyvor license file inside the container.
pub const DEFAULT_LICENSE_PATH: &str = "/etc/zeus/license/license.zyvor";

/// Returns the configured license file path.
/// Reads `ZEUS_LICENSE_PATH` env var; falls back to `DEFAULT_LICENSE_PATH`.
pub fn license_path() -> PathBuf {
    std::env::var("ZEUS_LICENSE_PATH")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LICENSE_PATH))
}

/// In-memory license state, held behind `Arc<RwLock<>>` in `AppState`.
#[derive(Debug, Clone)]
pub struct LicenseStore {
    /// Decoded claims; present only when a valid/loadable license file was found.
    pub claims: Option<LicenseClaims>,
    /// Last computed state (refreshed on load and on `/api/license/reload`).
    pub state: LicenseState,
    /// Last node count from the K8s API. `None` until first `/api/license/usage` call.
    pub last_node_count: Option<u32>,
    /// Path from which the license was loaded.
    pub path: PathBuf,
}

impl LicenseStore {
    /// Load and verify a license from `path`.
    ///
    /// Never returns `Err` — all failures map to `Missing`, `InvalidSignature`,
    /// or `WrongProduct` states so the server always starts regardless of license status.
    pub fn load_from_path(path: PathBuf) -> Self {
        if !path.exists() {
            tracing::info!("No license file at {:?} — state: MISSING", path);
            return Self {
                claims: None,
                state: LicenseState::Missing,
                last_node_count: None,
                path,
            };
        }

        match std::fs::read(&path) {
            Err(e) => {
                tracing::warn!("Cannot read license file {:?}: {}", path, e);
                Self {
                    claims: None,
                    state: LicenseState::Missing,
                    last_node_count: None,
                    path,
                }
            }
            Ok(bytes) => match verifier::verify_and_decode(&bytes) {
                Err(e) => {
                    tracing::warn!("License verification failed: {}", e);
                    let state = if e.to_string().contains("not for zeus-os") {
                        LicenseState::WrongProduct
                    } else {
                        LicenseState::InvalidSignature
                    };
                    Self {
                        claims: None,
                        state,
                        last_node_count: None,
                        path,
                    }
                }
                Ok(claims) => {
                    let state = compute_state(&claims, None);
                    tracing::info!(
                        "License loaded: {} ({}) — state: {}",
                        claims.customer,
                        claims.license_id,
                        state
                    );
                    Self {
                        claims: Some(claims),
                        state,
                        last_node_count: None,
                        path,
                    }
                }
            },
        }
    }

    /// Update `last_node_count` and recompute state from current node count.
    pub fn refresh_state(&mut self, current_nodes: Option<u32>) {
        self.last_node_count = current_nodes;
        if let Some(ref claims) = self.claims {
            self.state = compute_state(claims, current_nodes);
        }
    }
}

/// Shared type alias used in `AppState`.
pub type SharedLicense = Arc<RwLock<LicenseStore>>;

/// Emit a single audit event reflecting the current license state.
/// Called once from `start_server()` after the store is initialized.
pub fn emit_startup_audit(store: &LicenseStore) {
    use crate::audit::{ActionResult, AuditLog};
    let path = AuditLog::default_path();
    let mut log = AuditLog::load(&path).unwrap_or_default();

    let action = state_to_audit_action(&store.state);
    let result = match &store.state {
        LicenseState::Valid => ActionResult::Success,
        LicenseState::ExpiringSoon
        | LicenseState::ExpiredGrace
        | LicenseState::OverLimitGrace
        | LicenseState::Missing => ActionResult::Warning,
        _ => ActionResult::Failure,
    };
    let message = match &store.state {
        LicenseState::Valid => "License loaded and valid".to_string(),
        LicenseState::ExpiringSoon => "License expiring within 30 days".to_string(),
        LicenseState::ExpiredGrace => "License expired (warning mode)".to_string(),
        LicenseState::OverLimitGrace => "Node count exceeds license limit (warning mode)".to_string(),
        LicenseState::Missing => format!("No license file at {:?}", store.path),
        LicenseState::InvalidSignature => "License signature invalid or tampered".to_string(),
        LicenseState::WrongProduct => "License is for wrong product".to_string(),
        other => format!("License state: {}", other),
    };

    log.record(action, "zeus-os", Some("license"), result, &message, None);
    if let Err(e) = log.save(&path) {
        tracing::warn!("Failed to save audit log after license startup check: {}", e);
    }
}
