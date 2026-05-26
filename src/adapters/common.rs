// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Common utilities shared across CRD-based adapters (KubeVirt, Metal3)

use crate::runtime::{InstanceState, Status};
use kube::{
    api::{Api, ListParams},
    core::{DynamicObject, GroupVersionKind},
    discovery, Client,
};
use std::collections::{BTreeMap, HashMap};

/// Build standard managed labels for CRD resources
///
/// Always includes `app` and `managed-by` labels, plus any user-supplied labels.
pub fn build_managed_labels(
    name: &str,
    user_labels: &HashMap<String, String>,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    // Insert user labels first so that managed labels take precedence
    for (k, v) in user_labels {
        labels.insert(k.clone(), v.clone());
    }
    // Managed labels always win — insert after user labels to prevent overrides
    labels.insert("app".to_string(), name.to_string());
    labels.insert("managed-by".to_string(), "aether".to_string());
    labels
}

/// Parse CPU string (e.g., "4" or "2000m") to core count
pub fn parse_cpu_cores(cpu: &str) -> i32 {
    if cpu.ends_with('m') {
        match cpu.trim_end_matches('m').parse::<i32>() {
            Ok(milli) => (milli / 1000).max(1),
            Err(_) => {
                tracing::warn!("Invalid CPU value '{}', defaulting to 1 core", cpu);
                1
            }
        }
    } else {
        match cpu.parse::<i32>() {
            Ok(v) => v.max(1),
            Err(_) => {
                tracing::warn!("Invalid CPU value '{}', defaulting to 1 core", cpu);
                1
            }
        }
    }
}

/// Validate that a name is a valid Kubernetes DNS label (RFC 1123).
pub fn validate_kube_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty()
        || name.len() > 63
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        || name.starts_with('-')
        || name.ends_with('-')
    {
        anyhow::bail!(
            "Invalid Kubernetes name '{}': must be a valid DNS label (1-63 alphanumeric/hyphen chars, no leading/trailing hyphens)",
            name
        );
    }
    Ok(())
}

/// Build ListParams that filter to aether-managed resources
pub fn managed_list_params() -> ListParams {
    ListParams::default().labels("managed-by=aether")
}

/// Build a "not found" Status for a missing resource
pub fn not_found_status(entity: &str) -> Status {
    Status {
        state: InstanceState::Unknown,
        ready: false,
        message: Some(format!("{} not found", entity)),
        restart_count: 0,
    }
}

/// Discover and return an API handle for a CRD type
pub async fn discover_crd_api(
    client: Client,
    namespace: &str,
    group: &str,
    version: &str,
    kind: &str,
) -> anyhow::Result<Api<DynamicObject>> {
    let gvk = GroupVersionKind::gvk(group, version, kind);
    let disc = discovery::Discovery::new(client.clone()).run().await?;

    let apigroup = disc
        .groups()
        .find(|g| g.name() == gvk.group)
        .ok_or_else(|| anyhow::anyhow!("Cannot find {} API group", group))?;

    let (ar, _caps) = apigroup
        .recommended_kind(&gvk.kind)
        .ok_or_else(|| anyhow::anyhow!("Cannot find {} resource", kind))?;

    Ok(Api::namespaced_with(client, namespace, &ar))
}

/// Retry an async operation with exponential backoff.
/// Retries on transient errors, fails immediately on permanent errors.
pub async fn retry_with_backoff<F, Fut, T>(
    op_name: &str,
    max_retries: u32,
    initial_backoff_ms: u64,
    mut op: F,
) -> crate::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = crate::Result<T>>,
{
    let mut attempt = 0;
    loop {
        match op().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt > max_retries {
                    tracing::error!("{} failed after {} attempts: {}", op_name, attempt, e);
                    return Err(e);
                }
                let backoff = initial_backoff_ms * 2u64.pow(attempt - 1);
                tracing::warn!(
                    "{} failed (attempt {}/{}), retrying in {}ms: {}",
                    op_name, attempt, max_retries, backoff, e
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_cpu_cores ─────────────────────────────────────────────

    #[test]
    fn test_parse_cpu_cores_whole() {
        assert_eq!(parse_cpu_cores("4"), 4);
        assert_eq!(parse_cpu_cores("1"), 1);
        assert_eq!(parse_cpu_cores("16"), 16);
    }

    #[test]
    fn test_parse_cpu_cores_millicore() {
        assert_eq!(parse_cpu_cores("2000m"), 2);
        assert_eq!(parse_cpu_cores("4000m"), 4);
        assert_eq!(parse_cpu_cores("500m"), 1); // min 1 core
    }

    #[test]
    fn test_parse_cpu_cores_invalid() {
        assert_eq!(parse_cpu_cores("invalid"), 1); // defaults to 1
        assert_eq!(parse_cpu_cores("xm"), 1); // invalid millicore defaults
    }

    #[test]
    fn test_parse_cpu_cores_large_millicore() {
        assert_eq!(parse_cpu_cores("8000m"), 8);
        assert_eq!(parse_cpu_cores("16000m"), 16);
    }

    #[test]
    fn test_parse_cpu_cores_zero_millicore_clamps_to_one() {
        assert_eq!(parse_cpu_cores("0m"), 1);
    }

    #[test]
    fn test_parse_cpu_cores_single_core() {
        assert_eq!(parse_cpu_cores("1000m"), 1);
    }

    // ── build_managed_labels ────────────────────────────────────────

    #[test]
    fn test_build_managed_labels_base() {
        let labels = build_managed_labels("my-app", &HashMap::new());
        assert_eq!(labels.get("app").unwrap(), "my-app");
        assert_eq!(labels.get("managed-by").unwrap(), "aether");
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn test_build_managed_labels_with_user_labels() {
        let mut user = HashMap::new();
        user.insert("env".to_string(), "production".to_string());
        user.insert("team".to_string(), "infra".to_string());

        let labels = build_managed_labels("my-app", &user);
        assert_eq!(labels.get("app").unwrap(), "my-app");
        assert_eq!(labels.get("managed-by").unwrap(), "aether");
        assert_eq!(labels.get("env").unwrap(), "production");
        assert_eq!(labels.get("team").unwrap(), "infra");
        assert_eq!(labels.len(), 4);
    }

    #[test]
    fn test_build_managed_labels_user_cannot_override_managed_by() {
        let mut user = HashMap::new();
        user.insert("managed-by".to_string(), "someone-else".to_string());

        let labels = build_managed_labels("my-app", &user);
        // Managed labels are inserted after user labels, so they take precedence.
        // Users cannot override reserved keys like "managed-by".
        assert_eq!(labels.get("managed-by").unwrap(), "aether");
    }

    // ── managed_list_params ─────────────────────────────────────────

    #[test]
    fn test_managed_list_params_has_label_selector() {
        let lp = managed_list_params();
        assert_eq!(lp.label_selector.as_deref(), Some("managed-by=aether"));
    }

    // ── not_found_status ────────────────────────────────────────────

    #[test]
    fn test_not_found_status_vm() {
        let s = not_found_status("VM");
        assert_eq!(s.state, InstanceState::Unknown);
        assert!(!s.ready);
        assert_eq!(s.message.as_deref(), Some("VM not found"));
        assert_eq!(s.restart_count, 0);
    }

    #[test]
    fn test_not_found_status_host() {
        let s = not_found_status("Host");
        assert_eq!(s.message.as_deref(), Some("Host not found"));
    }
}
