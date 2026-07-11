// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! TLS certificate lifecycle: inspection and self-managed regeneration.
//!
//! Aether serves TLS with an operator-provided cert/key. It cannot renew a
//! CA-issued (externally-managed) certificate — that belongs to cert-manager /
//! ACME / your PKI. What it *can* own is a **self-signed** cert it regenerates
//! itself. This module distinguishes the two (an ownership model mirroring
//! secret rotation) so automation only ever regenerates certs Aether owns and
//! defers everything else to an external system.
//!
//! Regeneration writes a fresh cert/key to the same paths via `openssl` (which
//! ships in the deploy image and is how these certs are created operationally).
//! It takes effect on the next process start — the running TLS listener does not
//! hot-reload — so it suits systemd/bare deployments that restart on a timer or
//! supervisor. On Kubernetes, prefer cert-manager and a mounted Secret.

use anyhow::{Context, Result};
use std::path::Path;

/// Parsed facts about a TLS certificate on disk.
#[derive(Debug, Clone)]
pub struct CertInfo {
    /// `notAfter` as a Unix timestamp.
    pub not_after_epoch: i64,
    /// Whole days until expiry (negative once expired).
    pub days_left: i64,
    /// Issuer == subject ⇒ Aether can regenerate it.
    pub self_signed: bool,
    /// Full subject (RFC 2253).
    pub subject: String,
    /// Subject common name, if present (used as the regenerated cert's CN).
    pub common_name: Option<String>,
    /// Subject alternative names in openssl form, e.g. `DNS:host`, `IP:127.0.0.1`.
    pub sans: Vec<String>,
}

/// Inspect a PEM certificate file.
pub fn inspect(cert_path: &Path) -> Result<CertInfo> {
    let bytes = std::fs::read(cert_path)
        .with_context(|| format!("reading certificate {}", cert_path.display()))?;
    let (_, pem) = x509_parser::pem::parse_x509_pem(&bytes)
        .map_err(|e| anyhow::anyhow!("parsing PEM {}: {e}", cert_path.display()))?;
    let cert = pem
        .parse_x509()
        .map_err(|e| anyhow::anyhow!("parsing X.509 {}: {e}", cert_path.display()))?;

    let not_after_epoch = cert.validity().not_after.timestamp();
    let days_left = (not_after_epoch - chrono::Utc::now().timestamp()) / 86_400;
    let subject = cert.subject().to_string();
    let self_signed = cert.issuer().to_string() == subject;
    let common_name = cert
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .map(|s| s.to_string());

    let mut sans = Vec::new();
    if let Ok(Some(ext)) = cert.subject_alternative_name() {
        use x509_parser::extensions::GeneralName;
        for name in &ext.value.general_names {
            match name {
                GeneralName::DNSName(dns) => sans.push(format!("DNS:{dns}")),
                GeneralName::IPAddress(bytes) => {
                    if let Some(ip) = ip_from_bytes(bytes) {
                        sans.push(format!("IP:{ip}"));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(CertInfo {
        not_after_epoch,
        days_left,
        self_signed,
        subject,
        common_name,
        sans,
    })
}

fn ip_from_bytes(bytes: &[u8]) -> Option<String> {
    match bytes.len() {
        4 => Some(format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])),
        16 => {
            let segs: Vec<String> = bytes
                .chunks(2)
                .map(|c| format!("{:x}", ((c[0] as u16) << 8) | c[1] as u16))
                .collect();
            Some(segs.join(":"))
        }
        _ => None,
    }
}

/// Regenerate a self-signed cert/key pair at the given paths, preserving the
/// original's subject CN and SANs, valid for `validity_days`. Only call for a
/// cert whose [`CertInfo::self_signed`] is true. Uses `openssl`; returns an
/// error if `openssl` is unavailable or the command fails. The private key is
/// written `0600`.
pub fn regenerate_self_signed(
    cert_path: &Path,
    key_path: &Path,
    info: &CertInfo,
    validity_days: u32,
) -> Result<()> {
    anyhow::ensure!(
        info.self_signed,
        "refusing to regenerate a non-self-signed (externally-managed) certificate"
    );
    let cn = info.common_name.as_deref().unwrap_or("aether");
    let subj = format!("/CN={cn}");

    let mut cmd = std::process::Command::new("openssl");
    cmd.arg("req")
        .arg("-x509")
        .arg("-newkey")
        .arg("rsa:2048")
        .arg("-keyout")
        .arg(key_path)
        .arg("-out")
        .arg(cert_path)
        .arg("-days")
        .arg(validity_days.max(1).to_string())
        .arg("-nodes")
        .arg("-subj")
        .arg(&subj);
    if !info.sans.is_empty() {
        cmd.arg("-addext")
            .arg(format!("subjectAltName={}", info.sans.join(",")));
    }

    let output = cmd
        .output()
        .context("running openssl to regenerate self-signed certificate")?;
    anyhow::ensure!(
        output.status.success(),
        "openssl failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Self-signed CN=aether.test, SANs aether.test/localhost/127.0.0.1,
    // valid until 2126.
    const FIXTURE: &str = "-----BEGIN CERTIFICATE-----
MIIDODCCAiCgAwIBAgIUVYHRs9HKOk5fpIxDbJ6rBrB7huYwDQYJKoZIhvcNAQEL
BQAwFjEUMBIGA1UEAwwLYWV0aGVyLnRlc3QwIBcNMjYwNzExMTc0NjQ1WhgPMjEy
NjA2MTcxNzQ2NDVaMBYxFDASBgNVBAMMC2FldGhlci50ZXN0MIIBIjANBgkqhkiG
9w0BAQEFAAOCAQ8AMIIBCgKCAQEAxzfdKt9rtBGn4wJghK7zhPooz3IXDoDxAf1P
4QFn3V84ixnVBdvQFTmizEXdkJUQwV9T/y6VcJvSrRxNIxe8BYLHGRgoTEBcf+81
0IZu6dRN7dVR7eQ5UeTmKA4rNhsYpNLsDJO5aOwHKXxkAW+9GYzDD51i5t/xErii
EOyKJJK0GvRURXywVO2NPZc4OieoDXbql2tEHHwLf3fCoyS3AaDfBk16M71+9w61
EM8WeEHu2XccqJf7rbWwNMKF57qbC/V57zI2ndV+sU6Nq9vmZt9Tchfag5xCmsuV
qAy5eaQDiYjzA8Ni7Zz93gP3SlC7Po7AGo/9Sq+SYtP6PDqGwwIDAQABo3wwejAd
BgNVHQ4EFgQUvVGk1tt0uoqY+AqtOIJZETWXcJMwHwYDVR0jBBgwFoAUvVGk1tt0
uoqY+AqtOIJZETWXcJMwDwYDVR0TAQH/BAUwAwEB/zAnBgNVHREEIDAeggthZXRo
ZXIudGVzdIIJbG9jYWxob3N0hwR/AAABMA0GCSqGSIb3DQEBCwUAA4IBAQAEKJTI
llm13C5DWuUa6JRVCEGAy5U40V6N4kxfqGPmupTJ4/4VHOgKRP70b1fE0NtvJYjB
0MRwLIP0B0Cz2B87MELh0UAcMZpw22eetWy2U3ymxdsWzGlu/aEDiTGZHQo9SkSv
G2sHVPde/QhK14lpbb+ZB4V5jkRYuGLZIjY5ce0tMz25nmIVjjqctmuRFfzfQjZF
RENEJ1mKFAr1B7DFxaoFjB4GznsvRK8YeP8Xskk0bm4CUVLHLHkyNpIpsi3Tplm2
meMcnulebmWlByVGEilzUzC+TG1FULI3TdolAgw+hAA7sdBFQ301zw3/hG5QXfb9
BLIBm+mXYYL/PU/m
-----END CERTIFICATE-----
";

    #[test]
    fn inspect_self_signed_fixture() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cert.pem");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(FIXTURE.as_bytes())
            .unwrap();

        let info = inspect(&path).unwrap();
        assert!(info.self_signed, "issuer==subject ⇒ self-signed");
        assert!(info.days_left > 0, "fixture valid for decades");
        assert_eq!(info.common_name.as_deref(), Some("aether.test"));
        assert!(info.subject.contains("aether.test"));
        assert!(info.sans.contains(&"DNS:aether.test".to_string()));
        assert!(info.sans.contains(&"DNS:localhost".to_string()));
        assert!(info.sans.contains(&"IP:127.0.0.1".to_string()));
    }

    #[test]
    fn regenerate_refuses_non_self_signed() {
        let dir = tempfile::tempdir().unwrap();
        let info = CertInfo {
            not_after_epoch: 0,
            days_left: 3,
            self_signed: false,
            subject: "CN=external".into(),
            common_name: Some("external".into()),
            sans: vec![],
        };
        let err = regenerate_self_signed(
            &dir.path().join("c.pem"),
            &dir.path().join("k.pem"),
            &info,
            365,
        )
        .unwrap_err();
        assert!(err.to_string().contains("externally-managed"));
    }
}
