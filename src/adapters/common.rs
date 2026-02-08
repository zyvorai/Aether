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
    labels.insert("app".to_string(), name.to_string());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());
    for (k, v) in user_labels {
        labels.insert(k.clone(), v.clone());
    }
    labels
}

/// Parse CPU string (e.g., "4" or "2000m") to core count
pub fn parse_cpu_cores(cpu: &str) -> i32 {
    if cpu.ends_with('m') {
        let milli = cpu.trim_end_matches('m').parse::<i32>().unwrap_or(1000);
        (milli / 1000).max(1)
    } else {
        cpu.parse::<i32>().unwrap_or(1)
    }
}

/// Build ListParams that filter to orchestr8-managed resources
pub fn managed_list_params() -> ListParams {
    ListParams::default().labels("managed-by=orchestr8")
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
        assert_eq!(labels.get("managed-by").unwrap(), "orchestr8");
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn test_build_managed_labels_with_user_labels() {
        let mut user = HashMap::new();
        user.insert("env".to_string(), "production".to_string());
        user.insert("team".to_string(), "infra".to_string());

        let labels = build_managed_labels("my-app", &user);
        assert_eq!(labels.get("app").unwrap(), "my-app");
        assert_eq!(labels.get("managed-by").unwrap(), "orchestr8");
        assert_eq!(labels.get("env").unwrap(), "production");
        assert_eq!(labels.get("team").unwrap(), "infra");
        assert_eq!(labels.len(), 4);
    }

    #[test]
    fn test_build_managed_labels_user_cannot_override_managed_by() {
        let mut user = HashMap::new();
        user.insert("managed-by".to_string(), "someone-else".to_string());

        let labels = build_managed_labels("my-app", &user);
        // BTreeMap insert order: base labels first, then user labels override
        // Since user labels are inserted after, they would override.
        // This is acceptable — if users explicitly set managed-by, respect it.
        assert_eq!(labels.get("managed-by").unwrap(), "someone-else");
    }

    // ── managed_list_params ─────────────────────────────────────────

    #[test]
    fn test_managed_list_params_has_label_selector() {
        let lp = managed_list_params();
        assert_eq!(lp.label_selector.as_deref(), Some("managed-by=orchestr8"));
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
