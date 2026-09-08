// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Reconstruct logical **applications** from discovered Kubernetes objects and
//! assign each a migration class.

use crate::discovery::kubernetes::{DiscoveredWorkload, RawInventory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Migration class (vision §5, A–F).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationClass {
    /// A — stateless and portable.
    StatelessPortable,
    /// B — stateful but Kubernetes-native (uses PVCs).
    StatefulNative,
    /// C — depends on a cloud-managed service (RDS/S3/…).
    CloudManagedDependency,
    /// D — privileged or node-dependent (hostNetwork/hostPath/GPU/…).
    PrivilegedNodeDependent,
    /// E — operator-managed (owned by a CRD).
    OperatorManaged,
    /// F — non-portable/broken (naked pod, :latest, missing requests).
    NonPortable,
}

impl MigrationClass {
    pub fn letter(&self) -> char {
        match self {
            MigrationClass::StatelessPortable => 'A',
            MigrationClass::StatefulNative => 'B',
            MigrationClass::CloudManagedDependency => 'C',
            MigrationClass::PrivilegedNodeDependent => 'D',
            MigrationClass::OperatorManaged => 'E',
            MigrationClass::NonPortable => 'F',
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MigrationClass::StatelessPortable => "Stateless / portable",
            MigrationClass::StatefulNative => "Stateful (K8s-native)",
            MigrationClass::CloudManagedDependency => "Cloud-managed dependency",
            MigrationClass::PrivilegedNodeDependent => "Privileged / node-dependent",
            MigrationClass::OperatorManaged => "Operator-managed",
            MigrationClass::NonPortable => "Non-portable / needs remediation",
        }
    }
}

impl std::fmt::Display for MigrationClass {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} — {}", self.letter(), self.label())
    }
}

/// A logical application: a group of discovered objects that deploy together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub connection: String,
    /// `{kind}/{name}` of member workloads.
    pub workloads: Vec<String>,
    pub services: Vec<String>,
    pub pvcs: Vec<String>,
    pub config_map_refs: Vec<String>,
    pub secret_refs: Vec<String>,
    pub external_deps: Vec<String>,
    pub class: MigrationClass,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

/// Application-grouping label keys, in precedence order.
const PART_OF: &str = "app.kubernetes.io/part-of";
const APP_NAME: &str = "app.kubernetes.io/name";
const APP_INSTANCE: &str = "app.kubernetes.io/instance";
const HELM_RELEASE: &str = "meta.helm.sh/release-name";

/// Determine the grouping key for a workload (precedence: part-of › name/instance
/// › Helm release › root owner › the workload's own name).
fn group_key(w: &DiscoveredWorkload) -> String {
    if let Some(v) = w.labels.get(PART_OF).filter(|v| !v.is_empty()) {
        return v.clone();
    }
    if let Some(v) = w
        .labels
        .get(APP_NAME)
        .or_else(|| w.labels.get(APP_INSTANCE))
        .filter(|v| !v.is_empty())
    {
        return v.clone();
    }
    if let Some(v) = w.annotations.get(HELM_RELEASE).filter(|v| !v.is_empty()) {
        return v.clone();
    }
    // Root owner (e.g. a StatefulSet owning pods) — use the top owner's name.
    if let Some(o) = w.owner_refs.first() {
        if !o.name.is_empty() {
            return o.name.clone();
        }
    }
    w.name.clone()
}

/// True when `selector` is a non-empty subset of `labels` (a Service targets a pod).
fn selector_matches(
    selector: &BTreeMap<String, String>,
    labels: &BTreeMap<String, String>,
) -> bool {
    !selector.is_empty() && selector.iter().all(|(k, v)| labels.get(k) == Some(v))
}

/// Group a raw inventory into applications, classifying each.
pub fn group_into_applications(inv: &RawInventory) -> Vec<Application> {
    // Bucket workloads by (namespace, group-key).
    let mut buckets: BTreeMap<(String, String), Vec<&DiscoveredWorkload>> = BTreeMap::new();
    for w in &inv.workloads {
        buckets
            .entry((w.namespace.clone(), group_key(w)))
            .or_default()
            .push(w);
    }

    let mut apps = Vec::new();
    for ((namespace, key), members) in buckets {
        let mut app = Application {
            id: format!("{}/{}", namespace, key),
            name: key.clone(),
            namespace: namespace.clone(),
            connection: inv.connection.clone(),
            workloads: members
                .iter()
                .map(|w| format!("{}/{}", w.kind, w.name))
                .collect(),
            services: Vec::new(),
            pvcs: Vec::new(),
            config_map_refs: Vec::new(),
            secret_refs: Vec::new(),
            external_deps: Vec::new(),
            class: MigrationClass::StatelessPortable,
            blockers: Vec::new(),
            warnings: Vec::new(),
        };

        for w in &members {
            app.pvcs.extend(w.pvc_refs.iter().cloned());
            app.config_map_refs
                .extend(w.config_map_refs.iter().cloned());
            app.secret_refs.extend(w.secret_refs.iter().cloned());
            app.external_deps.extend(w.env_endpoints.iter().cloned());
        }

        // Attach services whose selector matches a member workload's labels.
        for svc in inv.services.iter().filter(|s| s.namespace == namespace) {
            if members
                .iter()
                .any(|w| selector_matches(&svc.selector, &w.labels))
            {
                app.services.push(svc.name.clone());
            }
        }

        dedup(&mut app.pvcs);
        dedup(&mut app.config_map_refs);
        dedup(&mut app.secret_refs);
        dedup(&mut app.external_deps);
        dedup(&mut app.services);

        let (class, blockers, warnings) = classify(&members);
        app.class = class;
        app.blockers = blockers;
        app.warnings = warnings;

        apps.push(app);
    }
    apps.sort_by(|a, b| a.id.cmp(&b.id));
    apps
}

fn dedup(v: &mut Vec<String>) {
    v.sort();
    v.dedup();
}

/// Known cloud-managed-service endpoint fragments (Class C signal).
const CLOUD_ENDPOINTS: &[&str] = &[
    "rds.amazonaws.com",
    "cache.amazonaws.com",
    "amazonaws.com",
    "database.azure.com",
    "blob.core.windows.net",
    "servicebus.windows.net",
    "cloudsql",
    "googleapis.com",
    "database.windows.net",
];

/// Classify a group of workloads into a migration class, collecting blockers.
fn classify(members: &[&DiscoveredWorkload]) -> (MigrationClass, Vec<String>, Vec<String>) {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    let mut node_dependent = false;
    let mut operator_managed = false;
    let mut has_pvc = false;
    let mut cloud_dep = false;
    let mut naked_pod = false;

    let controller_kinds = [
        "ReplicaSet",
        "Deployment",
        "StatefulSet",
        "DaemonSet",
        "Job",
        "CronJob",
        "",
    ];

    for w in members {
        if w.signals.host_network
            || w.signals.host_pid
            || !w.signals.host_path_volumes.is_empty()
            || w.signals.node_name.is_some()
            || w.containers.iter().any(|c| c.privileged || c.wants_gpu)
        {
            node_dependent = true;
        }
        if w.owner_refs
            .iter()
            .any(|o| !controller_kinds.contains(&o.kind.as_str()))
        {
            operator_managed = true;
        }
        if !w.pvc_refs.is_empty() {
            has_pvc = true;
        }
        if w.env_endpoints
            .iter()
            .any(|e| CLOUD_ENDPOINTS.iter().any(|c| e.contains(c)))
        {
            cloud_dep = true;
        }
        if w.kind == "Pod" && w.owner_refs.is_empty() {
            naked_pod = true;
            blockers.push(format!("naked Pod '{}' has no controller", w.name));
        }
        for c in &w.containers {
            if c.image.ends_with(":latest") || !c.image.contains(':') {
                blockers.push(format!(
                    "container '{}' uses a mutable tag ({})",
                    c.name, c.image
                ));
            }
            if !c.has_requests {
                warnings.push(format!("container '{}' has no resource requests", c.name));
            }
        }
    }

    // Precedence: node-dependent › operator › cloud-dep › stateful › non-portable › stateless.
    let class = if node_dependent {
        MigrationClass::PrivilegedNodeDependent
    } else if operator_managed {
        MigrationClass::OperatorManaged
    } else if cloud_dep {
        MigrationClass::CloudManagedDependency
    } else if has_pvc {
        MigrationClass::StatefulNative
    } else if naked_pod {
        MigrationClass::NonPortable
    } else {
        MigrationClass::StatelessPortable
    };

    dedup(&mut blockers);
    dedup(&mut warnings);
    (class, blockers, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::kubernetes::{ContainerInfo, OwnerRef, PodSignals};

    fn wl(kind: &str, name: &str) -> DiscoveredWorkload {
        DiscoveredWorkload {
            namespace: "prod".into(),
            kind: kind.into(),
            name: name.into(),
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            owner_refs: vec![],
            replicas: 1,
            containers: vec![ContainerInfo {
                name: "c".into(),
                image: "app:1.0".into(),
                has_requests: true,
                cpu_request: Some("100m".into()),
                memory_request: Some("128Mi".into()),
                wants_gpu: false,
                privileged: false,
            }],
            signals: PodSignals::default(),
            config_map_refs: vec![],
            secret_refs: vec![],
            pvc_refs: vec![],
            env_endpoints: vec![],
            manifest: None,
        }
    }

    #[test]
    fn test_grouping_by_part_of() {
        let mut a = wl("Deployment", "frontend");
        a.labels.insert(PART_OF.into(), "shop".into());
        let mut b = wl("Deployment", "api");
        b.labels.insert(PART_OF.into(), "shop".into());
        let inv = RawInventory {
            connection: "c".into(),
            workloads: vec![a, b],
            ..Default::default()
        };
        let apps = group_into_applications(&inv);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "shop");
        assert_eq!(apps[0].workloads.len(), 2);
    }

    #[test]
    fn test_class_stateful() {
        let mut w = wl("StatefulSet", "pg");
        w.pvc_refs.push("data".into());
        let inv = RawInventory {
            workloads: vec![w],
            ..Default::default()
        };
        let apps = group_into_applications(&inv);
        assert_eq!(apps[0].class, MigrationClass::StatefulNative);
    }

    #[test]
    fn test_class_node_dependent_and_cloud() {
        let mut w = wl("DaemonSet", "agent");
        w.signals.host_network = true;
        let inv = RawInventory {
            workloads: vec![w],
            ..Default::default()
        };
        assert_eq!(
            group_into_applications(&inv)[0].class,
            MigrationClass::PrivilegedNodeDependent
        );

        let mut c = wl("Deployment", "api");
        c.env_endpoints
            .push("db.abc.us-east-1.rds.amazonaws.com:5432".into());
        let inv = RawInventory {
            workloads: vec![c],
            ..Default::default()
        };
        assert_eq!(
            group_into_applications(&inv)[0].class,
            MigrationClass::CloudManagedDependency
        );
    }

    #[test]
    fn test_blockers_latest_and_naked_pod() {
        let mut w = wl("Pod", "debug");
        w.containers[0].image = "nginx:latest".into();
        let inv = RawInventory {
            workloads: vec![w],
            ..Default::default()
        };
        let app = &group_into_applications(&inv)[0];
        assert_eq!(app.class, MigrationClass::NonPortable);
        assert!(app.blockers.iter().any(|b| b.contains("naked Pod")));
        assert!(app.blockers.iter().any(|b| b.contains("mutable tag")));
    }

    #[test]
    fn test_operator_managed() {
        let mut w = wl("Deployment", "kafka");
        w.owner_refs.push(OwnerRef {
            kind: "Kafka".into(),
            name: "my-kafka".into(),
        });
        let inv = RawInventory {
            workloads: vec![w],
            ..Default::default()
        };
        assert_eq!(
            group_into_applications(&inv)[0].class,
            MigrationClass::OperatorManaged
        );
    }
}
