//! Trusted Execution Environment capability detection.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeeKind {
    Sev,
    SevEs,
    SevSnp,
    Tdx,
}

/// How Aether connects to Ragnarok confidential services.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RagnarokIntegration {
    /// `embedded` — in-process `src/ragnarok/` inside `aether serve`.
    /// `composite` — remote Ragnarok via `RAGNAROK_URL`.
    pub mode: String,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeeCapabilities {
    pub host: HostTeeStatus,
    pub clusters: Vec<ClusterTeeStatus>,
    #[serde(default)]
    pub integration: RagnarokIntegration,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HostTeeStatus {
    pub sev_device: bool,
    pub sev_snp: bool,
    pub tdx: bool,
    pub dev_sev_path: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTeeStatus {
    pub cluster: String,
    pub nodes_with_snp: u32,
    pub nodes_with_tdx: u32,
    pub total_nodes: u32,
    pub labels: Vec<String>,
}

/// Probe local host for AMD SEV / Intel TDX indicators.
pub fn probe_host_tee() -> HostTeeStatus {
    let mut status = HostTeeStatus::default();
    let sev_path = Path::new("/dev/sev");
    let sev_guest_path = Path::new("/dev/sev-guest");

    if sev_path.exists() || sev_guest_path.exists() {
        status.sev_device = true;
        status.dev_sev_path = Some(
            if sev_guest_path.exists() {
                "/dev/sev-guest".into()
            } else {
                "/dev/sev".into()
            },
        );
    }

    if std::env::var("AETHER_TEE_SNP")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        status.sev_snp = true;
        status.notes.push("SNP enabled via AETHER_TEE_SNP".into());
    } else if status.sev_device {
        status.sev_snp = probe_snp_from_proc();
    }

    if std::env::var("AETHER_TEE_TDX")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || Path::new("/sys/firmware/tdx/seam").exists()
    {
        status.tdx = true;
    }

    if !status.sev_device && !status.tdx {
        status.notes.push(
            "No TEE devices detected — use AETHER_TEE_SNP=1 for dev simulation".into(),
        );
    }

    status
}

fn probe_snp_from_proc() -> bool {
    std::fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.to_lowercase().contains("sev_snp") || s.contains("snp"))
        .unwrap_or(false)
}

pub fn tee_node_selector(tee: &crate::spec::ConfidentialTee) -> Option<std::collections::HashMap<String, String>> {
    use crate::spec::ConfidentialTee;
    use std::collections::HashMap;
    let mut sel = HashMap::new();
    match tee {
        ConfidentialTee::SevSnp => {
            sel.insert("node.kubernetes.io/sev-snp".into(), "true".into());
        }
        ConfidentialTee::Sev | ConfidentialTee::SevEs => {
            sel.insert("feature.node.kubernetes.io/sev".into(), "true".into());
        }
        ConfidentialTee::Tdx => {
            sel.insert("feature.node.kubernetes.io/tdx".into(), "true".into());
        }
    }
    Some(sel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_host_tee_runs() {
        let s = probe_host_tee();
        assert!(s.notes.len() >= 1 || s.sev_device || s.tdx);
    }
}
