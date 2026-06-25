// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::claims::LicenseClaims;
use super::enforcement::LicenseState;
use super::usage::ManagedNodeInfo;

/// Response for `GET /api/license/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatusResponse {
    pub state: String,
    pub license_id: Option<String>,
    pub customer: Option<String>,
    pub customer_id: Option<String>,
    pub product: Option<String>,
    pub allowed_nodes: Option<u32>,
    pub allowed_clusters: Option<u32>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub issued_at: Option<String>,
    pub license_version: Option<u32>,
    pub days_remaining: Option<i64>,
    pub current_node_count: Option<u32>,
    pub warning_message: Option<String>,
}

/// Response for `GET /api/license/usage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseUsageResponse {
    pub allowed_nodes: u32,
    pub current_node_count: u32,
    pub utilization_pct: f64,
    pub nodes: Vec<ManagedNodeInfo>,
}

impl LicenseStatusResponse {
    pub fn from_state_and_claims(
        state: &LicenseState,
        claims: Option<&LicenseClaims>,
        current_nodes: Option<u32>,
    ) -> Self {
        let days_remaining = claims
            .and_then(|c| NaiveDate::parse_from_str(&c.valid_until, "%Y-%m-%d").ok())
            .map(|u| (u - Utc::now().date_naive()).num_days());

        let warning_message = match state {
            LicenseState::ExpiringSoon => days_remaining
                .map(|d| format!("License expires in {} day(s). Contact Zyvor to renew.", d)),
            LicenseState::ExpiredGrace => Some(
                "License has expired. Zeus OS is operating in warning mode. Contact Zyvor."
                    .to_string(),
            ),
            LicenseState::OverLimitGrace => current_nodes.zip(claims).map(|(n, c)| {
                format!(
                    "Node count {} exceeds licensed limit of {}. Contact Zyvor to increase capacity.",
                    n, c.allowed_nodes
                )
            }),
            LicenseState::Missing => Some(
                "No license file found. Set ZEUS_LICENSE_PATH or mount a license Secret."
                    .to_string(),
            ),
            LicenseState::InvalidSignature => {
                Some("License file has an invalid or tampered signature.".to_string())
            }
            LicenseState::WrongProduct => {
                Some("License file is not issued for zeus-os.".to_string())
            }
            _ => None,
        };

        Self {
            state: state.to_string(),
            license_id: claims.map(|c| c.license_id.clone()),
            customer: claims.map(|c| c.customer.clone()),
            customer_id: claims.map(|c| c.customer_id.clone()),
            product: claims.map(|c| c.product.clone()),
            allowed_nodes: claims.map(|c| c.allowed_nodes),
            allowed_clusters: claims.map(|c| c.allowed_clusters),
            valid_from: claims.map(|c| c.valid_from.clone()),
            valid_until: claims.map(|c| c.valid_until.clone()),
            issued_at: claims.map(|c| c.issued_at.clone()),
            license_version: claims.map(|c| c.license_version),
            days_remaining,
            current_node_count: current_nodes,
            warning_message,
        }
    }
}
