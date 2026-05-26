//! GuestKit offline inspection integration (pre-launch, policy, post-shutdown, repair).

use crate::ragnarok::image::ImageCatalog;
use crate::ragnarok::sovereign::offline_mode_active;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InspectionMode {
    PreLaunch,
    OfflinePolicy,
    PostShutdown,
    AttestedRepair,
}

impl InspectionMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "offline-policy" => Self::OfflinePolicy,
            "post-shutdown" => Self::PostShutdown,
            "attested-repair" => Self::AttestedRepair,
            _ => Self::PreLaunch,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreLaunch => "pre-launch",
            Self::OfflinePolicy => "offline-policy",
            Self::PostShutdown => "post-shutdown",
            Self::AttestedRepair => "attested-repair",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GuestKitPolicy {
    #[serde(default)]
    pub allowed_launch_digests: Vec<String>,
    #[serde(default)]
    pub allowed_signing_keys: Vec<String>,
    #[serde(default)]
    pub require_catalog_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestKitRequest {
    pub vm_id: String,
    pub image_path: Option<String>,
    pub mode: InspectionMode,
    pub policy_manifest: Option<String>,
    pub expected_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestKitResult {
    pub vm_id: String,
    pub mode: InspectionMode,
    pub passed: bool,
    pub findings: Vec<String>,
    pub chain_valid: bool,
    pub repair_steps: Vec<String>,
    pub inspected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestKitSummary {
    pub last_mode: String,
    pub passed: bool,
    pub findings: Vec<String>,
    pub repair_steps: Vec<String>,
    pub inspected_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GuestKitStore {
    inspections: HashMap<String, Vec<GuestKitResult>>,
}

#[derive(Clone)]
pub struct GuestKitService {
    path: PathBuf,
    store: Arc<Mutex<GuestKitStore>>,
}

impl GuestKitService {
    pub fn new(state_dir: PathBuf) -> Self {
        let path = state_dir.join("ragnarok-guestkit.json");
        let store = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            GuestKitStore::default()
        };
        Self {
            path,
            store: Arc::new(Mutex::new(store)),
        }
    }

    pub fn inspect(
        &self,
        req: &GuestKitRequest,
        catalog: &ImageCatalog,
    ) -> Result<GuestKitResult> {
        let policy = parse_policy(req.policy_manifest.as_deref())?;
        let mut findings = Vec::new();
        let mut repair_steps = Vec::new();
        let mut chain_valid = false;

        match req.mode {
            InspectionMode::PreLaunch => {
                chain_valid = validate_pre_launch(req, catalog, &policy, &mut findings)?;
            }
            InspectionMode::OfflinePolicy => {
                chain_valid = validate_offline_policy(&policy, req, &mut findings)?;
            }
            InspectionMode::PostShutdown => {
                chain_valid = validate_post_shutdown(req, catalog, &mut findings)?;
            }
            InspectionMode::AttestedRepair => {
                chain_valid = false;
                findings.push("Attestation or chain validation previously failed".into());
                repair_steps = repair_playbook(req, catalog, self.history(&req.vm_id));
            }
        }

        if offline_mode_active() && req.mode == InspectionMode::OfflinePolicy && policy.is_empty() {
            findings.push("Offline sovereign mode: embed policy manifest for air-gapped verify".into());
        }

        let passed = chain_valid && findings.iter().all(|f| !f.starts_with("ERROR:"));

        let result = GuestKitResult {
            vm_id: req.vm_id.clone(),
            mode: req.mode.clone(),
            passed,
            findings,
            chain_valid,
            repair_steps,
            inspected_at: chrono::Utc::now().to_rfc3339(),
        };

        {
            let mut store = self.store.lock().unwrap();
            store
                .inspections
                .entry(req.vm_id.clone())
                .or_default()
                .push(result.clone());
        }
        self.persist()?;
        Ok(result)
    }

    pub fn history(&self, vm_id: &str) -> Vec<GuestKitResult> {
        self.store
            .lock()
            .unwrap()
            .inspections
            .get(vm_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn summary(&self, vm_id: &str) -> Option<GuestKitSummary> {
        let history = self.history(vm_id);
        history.last().map(|r| GuestKitSummary {
            last_mode: r.mode.as_str().into(),
            passed: r.passed,
            findings: r.findings.clone(),
            repair_steps: r.repair_steps.clone(),
            inspected_at: r.inspected_at.clone(),
        })
    }

    fn persist(&self) -> Result<()> {
        let store = self.store.lock().unwrap();
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(&*store)?)?;
        Ok(())
    }
}

pub fn inspect(req: &GuestKitRequest) -> GuestKitResult {
    let dir = crate::ragnarok::client::RagnarokClient::attestation_data_dir();
    let svc = GuestKitService::new(dir.clone());
    let catalog = ImageCatalog::load(&dir);
    svc.inspect(req, &catalog).unwrap_or_else(|e| GuestKitResult {
        vm_id: req.vm_id.clone(),
        mode: req.mode.clone(),
        passed: false,
        findings: vec![format!("ERROR: {e}")],
        chain_valid: false,
        repair_steps: vec![],
        inspected_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn parse_policy(raw: Option<&str>) -> Result<GuestKitPolicy> {
    let Some(raw) = raw.filter(|s| !s.trim().is_empty()) else {
        return Ok(GuestKitPolicy::default());
    };
    serde_json::from_str(raw).context("invalid GuestKit policy manifest JSON")
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn validate_pre_launch(
    req: &GuestKitRequest,
    catalog: &ImageCatalog,
    policy: &GuestKitPolicy,
    findings: &mut Vec<String>,
) -> Result<bool> {
    if req.vm_id.is_empty() {
        bail!("vm_id required");
    }

    let mut chain_ok = false;

    if let Some(ref path) = req.image_path {
        let p = Path::new(path);
        let hash = hash_file(p)?;
        findings.push(format!("Image SHA-256: {hash}"));

        if policy.require_catalog_match || catalog.verify_digest(&hash) {
            if catalog.verify_digest(&hash) {
                findings.push("Image hash present in measured catalog".into());
                chain_ok = true;
            } else {
                findings.push("ERROR: image hash not in verified catalog — run image sign".into());
            }
        } else {
            chain_ok = true;
        }

        if let Some(ref expected) = req.expected_digest {
            if &hash == expected {
                findings.push("Expected launch digest matches image file".into());
            } else {
                findings.push("ERROR: image hash does not match spec confidential.imageDigest".into());
                chain_ok = false;
            }
        }
    } else if let Some(ref expected) = req.expected_digest {
        if catalog.verify_digest(expected) {
            findings.push("Spec launch digest found in measured catalog".into());
            chain_ok = true;
        } else {
            findings.push("ERROR: spec launch digest missing from catalog".into());
        }
    } else if !policy.allowed_launch_digests.is_empty() {
        findings.push("Policy manifest loaded; no image path to verify".into());
        chain_ok = true;
    } else {
        findings.push("ERROR: provide --image path or spec launch digest for pre-launch".into());
    }

    Ok(chain_ok)
}

fn validate_offline_policy(
    policy: &GuestKitPolicy,
    req: &GuestKitRequest,
    findings: &mut Vec<String>,
) -> Result<bool> {
    if policy.is_empty() && req.policy_manifest.is_none() {
        bail!("offline-policy mode requires policy manifest JSON");
    }
    findings.push(format!(
        "Offline policy: {} allowed digests, {} signing keys",
        policy.allowed_launch_digests.len(),
        policy.allowed_signing_keys.len()
    ));

    if let Some(ref digest) = req.expected_digest {
        if policy.allowed_launch_digests.is_empty() {
            findings.push("ERROR: offline policy has no allowed launch digests".into());
            return Ok(false);
        }
        let allowed = policy.allowed_launch_digests.iter().any(|d| d == digest);
        if allowed {
            findings.push("Launch digest permitted by offline policy".into());
        } else {
            findings.push("ERROR: launch digest not in offline policy allowlist".into());
            return Ok(false);
        }
    }

    Ok(true)
}

fn validate_post_shutdown(
    req: &GuestKitRequest,
    catalog: &ImageCatalog,
    findings: &mut Vec<String>,
) -> Result<bool> {
    findings.push("Post-shutdown inspection: encrypted snapshot chain review".into());
    if let Some(ref path) = req.image_path {
        let hash = hash_file(Path::new(path))?;
        findings.push(format!("Snapshot SHA-256: {hash}"));
        if catalog.verify_digest(&hash) {
            findings.push("Snapshot hash matches measured catalog entry".into());
            return Ok(true);
        }
        findings.push("Snapshot not in catalog — forensic review recommended".into());
        return Ok(false);
    }
    findings.push("No snapshot path — record VM shutdown event only".into());
    Ok(true)
}

fn repair_playbook(
    req: &GuestKitRequest,
    catalog: &ImageCatalog,
    history: Vec<GuestKitResult>,
) -> Vec<String> {
    let mut steps = vec![
        "Re-sign VM disk with `aether confidential image sign`".into(),
        "Update workload spec `confidential.imageDigest` to signed launch digest".into(),
        "Re-deploy workload and wait for attestation pass on Trust tab".into(),
    ];

    if let Some(ref digest) = req.expected_digest {
        if !catalog.verify_digest(digest) {
            steps.insert(
                0,
                format!("Register digest {digest} in catalog via image sign"),
            );
        }
    }

    if history.iter().any(|h| h.findings.iter().any(|f| f.contains("policy"))) {
        steps.push("Refresh RAGNAROK_TRUST_POLICY or embedded offline policy manifest".into());
    }
    steps.push("Revoke and re-release attest-gated secrets after successful re-attestation".into());
    steps
}

impl GuestKitPolicy {
    fn is_empty(&self) -> bool {
        self.allowed_launch_digests.is_empty()
            && self.allowed_signing_keys.is_empty()
            && !self.require_catalog_match
    }
}

pub fn enrich_explain(summary: Option<&GuestKitSummary>) -> Option<String> {
    summary.map(|s| {
        if s.passed {
            format!(
                "GuestKit {} inspection passed at {}",
                s.last_mode, s.inspected_at
            )
        } else {
            format!(
                "GuestKit {} failed: {}",
                s.last_mode,
                s.findings.join("; ")
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pre_launch_with_catalog_digest() {
        let dir = tempdir().unwrap();
        let catalog = ImageCatalog::load(dir.path());
        let image = dir.path().join("disk.raw");
        std::fs::write(&image, b"guestkit-test-image").unwrap();
        let manifest = catalog.sign("test-vm", &image, "cosign://aether").unwrap();

        let svc = GuestKitService::new(dir.path().to_path_buf());
        let result = svc
            .inspect(
                &GuestKitRequest {
                    vm_id: "vm-a".into(),
                    image_path: Some(image.display().to_string()),
                    mode: InspectionMode::PreLaunch,
                    policy_manifest: None,
                    expected_digest: manifest.launch_digest.clone(),
                },
                &catalog,
            )
            .unwrap();
        assert!(result.passed);
        assert!(result.chain_valid);
        assert_eq!(svc.history("vm-a").len(), 1);
    }

    #[test]
    fn offline_policy_rejects_digest() {
        let dir = tempdir().unwrap();
        let catalog = ImageCatalog::load(dir.path());
        let svc = GuestKitService::new(dir.path().to_path_buf());
        let policy = r#"{"allowedLaunchDigests":["allowed-only"]}"#;
        let result = svc
            .inspect(
                &GuestKitRequest {
                    vm_id: "vm-b".into(),
                    image_path: None,
                    mode: InspectionMode::OfflinePolicy,
                    policy_manifest: Some(policy.into()),
                    expected_digest: Some("bad-digest".into()),
                },
                &catalog,
            )
            .unwrap();
        assert!(!result.passed);
        assert!(result.findings.iter().any(|f| f.starts_with("ERROR:")));
    }

    #[test]
    fn attested_repair_emits_playbook() {
        let dir = tempdir().unwrap();
        let catalog = ImageCatalog::load(dir.path());
        let svc = GuestKitService::new(dir.path().to_path_buf());
        let result = svc
            .inspect(
                &GuestKitRequest {
                    vm_id: "vm-c".into(),
                    image_path: None,
                    mode: InspectionMode::AttestedRepair,
                    policy_manifest: None,
                    expected_digest: Some("missing".into()),
                },
                &catalog,
            )
            .unwrap();
        assert!(!result.passed);
        assert!(!result.repair_steps.is_empty());
    }
}
