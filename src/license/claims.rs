// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

use serde::{Deserialize, Serialize};

/// Decoded claims from a .zyvor license file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    pub license_id: String,
    pub product: String,
    pub customer: String,
    pub customer_id: String,
    pub allowed_nodes: u32,
    pub allowed_clusters: u32,
    pub valid_from: String,
    pub valid_until: String,
    pub issued_at: String,
    pub license_version: u32,
}

/// Outer signed envelope stored in the .zyvor file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyvorEnvelope {
    /// Must equal "zyvor-v1".
    pub format: String,
    /// base64url(utf8(json_claims))
    pub payload: String,
    /// base64url(rsa_pkcs1v15_sha256 over payload bytes)
    pub signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claims_roundtrip() {
        let c = LicenseClaims {
            license_id: "ZV-ZEUS-2026-0001".into(),
            product: "zeus-os".into(),
            customer: "Test Corp".into(),
            customer_id: "test-corp".into(),
            allowed_nodes: 10,
            allowed_clusters: 2,
            valid_from: "2026-01-01".into(),
            valid_until: "2027-01-01".into(),
            issued_at: "2026-01-01".into(),
            license_version: 1,
        };
        let json = serde_json::to_string(&c).unwrap();
        let c2: LicenseClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(c.license_id, c2.license_id);
        assert_eq!(c.allowed_nodes, c2.allowed_nodes);
    }

    #[test]
    fn envelope_roundtrip() {
        let e = ZyvorEnvelope {
            format: "zyvor-v1".into(),
            payload: "abc123".into(),
            signature: "sig456".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: ZyvorEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(e.format, e2.format);
        assert_eq!(e.payload, e2.payload);
    }
}
