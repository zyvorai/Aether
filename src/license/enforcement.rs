// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::claims::LicenseClaims;

/// All possible license states.
///
/// v0.3.0: enforcement is warn-only — *Blocked variants are defined for
/// future use but `compute_state` returns the *Grace variants instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseState {
    Valid,
    /// Less than 30 days until expiry.
    ExpiringSoon,
    /// License has expired (warn-only in v0.3.0).
    ExpiredGrace,
    /// Reserved for v0.4.0 hard block after grace period.
    ExpiredBlocked,
    /// Node count exceeds the licensed limit (warn-only in v0.3.0).
    OverLimitGrace,
    /// Reserved for v0.4.0 hard block after grace period.
    OverLimitBlocked,
    /// No license file found at the configured path.
    Missing,
    /// License file present but signature is invalid or tampered.
    InvalidSignature,
    /// License is signed for a different product.
    WrongProduct,
}

impl std::fmt::Display for LicenseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LicenseState::Valid => "VALID",
            LicenseState::ExpiringSoon => "EXPIRING_SOON",
            LicenseState::ExpiredGrace => "EXPIRED_GRACE",
            LicenseState::ExpiredBlocked => "EXPIRED_BLOCKED",
            LicenseState::OverLimitGrace => "OVER_LIMIT_GRACE",
            LicenseState::OverLimitBlocked => "OVER_LIMIT_BLOCKED",
            LicenseState::Missing => "MISSING",
            LicenseState::InvalidSignature => "INVALID_SIGNATURE",
            LicenseState::WrongProduct => "WRONG_PRODUCT",
        };
        write!(f, "{}", s)
    }
}

/// Compute the current license state from verified claims and a live node count.
///
/// `current_nodes` is `None` when the K8s API was unreachable; in that case
/// node-limit violations are not triggered.
pub fn compute_state(claims: &LicenseClaims, current_nodes: Option<u32>) -> LicenseState {
    let today = Utc::now().date_naive();

    let valid_from = match NaiveDate::parse_from_str(&claims.valid_from, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return LicenseState::InvalidSignature,
    };
    let valid_until = match NaiveDate::parse_from_str(&claims.valid_until, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return LicenseState::InvalidSignature,
    };

    if today < valid_from {
        return LicenseState::InvalidSignature;
    }

    if today > valid_until {
        // v0.3.0: warn only. v0.4.0 will use ExpiredBlocked.
        return LicenseState::ExpiredGrace;
    }

    let days_remaining = (valid_until - today).num_days();
    if days_remaining < 30 {
        return LicenseState::ExpiringSoon;
    }

    if let Some(count) = current_nodes {
        if count > claims.allowed_nodes {
            // v0.3.0: warn only. v0.4.0 will use OverLimitBlocked.
            return LicenseState::OverLimitGrace;
        }
    }

    LicenseState::Valid
}

/// Map a LicenseState to the corresponding audit action variant.
pub fn state_to_audit_action(state: &LicenseState) -> crate::audit::AuditAction {
    use crate::audit::AuditAction;
    match state {
        LicenseState::Valid | LicenseState::ExpiringSoon => AuditAction::LicenseValid,
        LicenseState::Missing => AuditAction::LicenseMissing,
        LicenseState::InvalidSignature | LicenseState::WrongProduct => {
            AuditAction::LicenseInvalidSignature
        }
        LicenseState::ExpiredGrace | LicenseState::ExpiredBlocked => AuditAction::LicenseExpired,
        LicenseState::OverLimitGrace | LicenseState::OverLimitBlocked => {
            AuditAction::LicenseOverLimit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(allowed_nodes: u32, valid_from: &str, valid_until: &str) -> LicenseClaims {
        LicenseClaims {
            license_id: "TEST-001".into(),
            product: "zeus-os".into(),
            customer: "Test".into(),
            customer_id: "test".into(),
            allowed_nodes,
            allowed_clusters: 3,
            valid_from: valid_from.into(),
            valid_until: valid_until.into(),
            issued_at: "2026-01-01".into(),
            license_version: 1,
        }
    }

    #[test]
    fn valid_license() {
        let c = claims(100, "2026-01-01", "2099-12-31");
        assert_eq!(compute_state(&c, Some(50)), LicenseState::Valid);
    }

    #[test]
    fn expiring_soon() {
        // Use a date 29 days from now
        let today = Utc::now().date_naive();
        let soon = (today + chrono::Duration::days(29)).format("%Y-%m-%d").to_string();
        let c = claims(100, "2026-01-01", &soon);
        assert_eq!(compute_state(&c, Some(50)), LicenseState::ExpiringSoon);
    }

    #[test]
    fn expired_grace() {
        let c = claims(100, "2020-01-01", "2021-01-01");
        assert_eq!(compute_state(&c, None), LicenseState::ExpiredGrace);
    }

    #[test]
    fn over_limit_grace() {
        let c = claims(5, "2026-01-01", "2099-12-31");
        assert_eq!(compute_state(&c, Some(6)), LicenseState::OverLimitGrace);
    }

    #[test]
    fn no_nodes_no_violation() {
        let c = claims(5, "2026-01-01", "2099-12-31");
        assert_eq!(compute_state(&c, None), LicenseState::Valid);
    }

    #[test]
    fn invalid_date_format() {
        let c = claims(100, "not-a-date", "2099-12-31");
        assert_eq!(compute_state(&c, None), LicenseState::InvalidSignature);
    }

    #[test]
    fn not_yet_active() {
        let c = claims(100, "2099-01-01", "2100-01-01");
        assert_eq!(compute_state(&c, None), LicenseState::InvalidSignature);
    }
}
