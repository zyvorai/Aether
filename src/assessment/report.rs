// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! The "Cloud Exit Assessment" customer report: assess every application in a
//! connection's inventory, group into migration waves, and render.

use crate::assessment::portability::{assess, PortabilityAssessment};
use crate::inventory::application::MigrationClass;
use crate::inventory::InventorySnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportTotals {
    pub applications: usize,
    pub stateful: usize,
    pub external_dependencies: usize,
    pub blockers: usize,
    pub average_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave {
    pub number: u8,
    pub title: String,
    pub applications: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentReport {
    pub connection: String,
    pub discovered_at: String,
    pub totals: ReportTotals,
    pub assessments: Vec<PortabilityAssessment>,
    pub waves: Vec<Wave>,
    /// Detected AWS managed-service dependencies and their K8s-native targets.
    #[serde(default)]
    pub aws_dependencies: Vec<crate::assessment::aws::AwsDependency>,
}

/// Assign a migration wave (0 = remediate first, higher = riskier/later).
fn wave_for(class: MigrationClass) -> (u8, &'static str) {
    match class {
        MigrationClass::NonPortable => (0, "Remediate first"),
        MigrationClass::StatelessPortable => (1, "Stateless services"),
        MigrationClass::StatefulNative => (3, "Stateful (K8s-native)"),
        MigrationClass::OperatorManaged => (4, "Operator-managed"),
        MigrationClass::CloudManagedDependency => (4, "Cloud-managed dependencies"),
        MigrationClass::PrivilegedNodeDependent => (5, "Privileged / node-dependent"),
    }
}

/// Build the assessment report for a discovery snapshot.
pub fn build_report(snapshot: &InventorySnapshot) -> AssessmentReport {
    let mut assessments: Vec<PortabilityAssessment> = snapshot
        .applications
        .iter()
        .filter_map(|app| assess(app, &snapshot.raw).ok())
        .collect();
    assessments.sort_by_key(|a| std::cmp::Reverse(a.score));

    let totals = ReportTotals {
        applications: assessments.len(),
        stateful: assessments
            .iter()
            .filter(|a| a.class == MigrationClass::StatefulNative)
            .count(),
        external_dependencies: snapshot
            .applications
            .iter()
            .map(|a| a.external_deps.len())
            .sum(),
        blockers: assessments.iter().map(|a| a.blockers.len()).sum(),
        average_score: if assessments.is_empty() {
            0
        } else {
            (assessments.iter().map(|a| a.score as u32).sum::<u32>() / assessments.len() as u32)
                as u8
        },
    };

    // Group into waves.
    let mut wave_map: std::collections::BTreeMap<u8, (String, Vec<String>)> = Default::default();
    for a in &assessments {
        let (n, title) = wave_for(a.class);
        wave_map
            .entry(n)
            .or_insert_with(|| (title.to_string(), Vec::new()))
            .1
            .push(a.application.clone());
    }
    let waves = wave_map
        .into_iter()
        .map(|(number, (title, applications))| Wave {
            number,
            title,
            applications,
        })
        .collect();

    let aws_dependencies = crate::assessment::aws::detect(&snapshot.applications);

    AssessmentReport {
        connection: snapshot.connection.clone(),
        discovered_at: snapshot.discovered_at.clone(),
        totals,
        assessments,
        waves,
        aws_dependencies,
    }
}

impl AssessmentReport {
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Render as a Markdown "Cloud Exit Assessment".
    pub fn to_markdown(&self) -> String {
        let mut m = String::new();
        m.push_str(&format!(
            "# Cloud Exit Assessment — {}\n\n",
            self.connection
        ));
        m.push_str(&format!("_Discovered: {}_\n\n", self.discovered_at));

        m.push_str("## Summary\n\n");
        m.push_str(&format!(
            "| Applications | Stateful | External deps | Blockers | Avg score |\n\
             |---|---|---|---|---|\n\
             | {} | {} | {} | {} | {}/100 |\n\n",
            self.totals.applications,
            self.totals.stateful,
            self.totals.external_dependencies,
            self.totals.blockers,
            self.totals.average_score
        ));

        m.push_str("## Applications\n\n");
        m.push_str(
            "| App | Namespace | Class | Score | Complexity | Target | Strategy | Blockers |\n",
        );
        m.push_str("|---|---|---|---|---|---|---|---|\n");
        for a in &self.assessments {
            m.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
                a.application,
                a.namespace,
                a.class.letter(),
                a.score,
                a.complexity,
                a.recommended_target,
                a.recommended_strategy,
                a.blockers.len()
            ));
        }
        m.push('\n');

        if !self.aws_dependencies.is_empty() {
            m.push_str("## AWS managed-service dependencies → Kubernetes targets\n\n");
            m.push_str("| App | AWS service | Endpoint | Kubernetes target | Migration method | Difficulty |\n");
            m.push_str("|---|---|---|---|---|---|\n");
            for d in &self.aws_dependencies {
                m.push_str(&format!(
                    "| {} | {} | `{}` | {} | {} | {} |\n",
                    d.application,
                    d.service.name(),
                    redact_endpoint(&d.endpoint),
                    d.service.k8s_target(),
                    d.service.method(),
                    d.service.difficulty(),
                ));
            }
            m.push('\n');
        }

        m.push_str("## Migration waves\n\n");
        for w in &self.waves {
            m.push_str(&format!(
                "- **Wave {} — {}**: {}\n",
                w.number,
                w.title,
                w.applications.join(", ")
            ));
        }
        m.push('\n');
        m
    }
}

/// Strip any credentials embedded in a discovered endpoint before printing it.
fn redact_endpoint(e: &str) -> String {
    // Drop a `user:pass@` prefix if present (e.g. postgres://u:p@host).
    if let Some(at) = e.rfind('@') {
        if let Some(scheme) = e.find("://") {
            return format!("{}://***@{}", &e[..scheme], &e[at + 1..]);
        }
    }
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::kubernetes::{DiscoveredWorkload, PodSignals, RawInventory};
    use std::collections::BTreeMap;

    fn snap() -> InventorySnapshot {
        let mut a = DiscoveredWorkload {
            namespace: "prod".into(),
            kind: "Deployment".into(),
            name: "web".into(),
            labels: {
                let mut l = BTreeMap::new();
                l.insert("app.kubernetes.io/part-of".into(), "shop".into());
                l
            },
            annotations: BTreeMap::new(),
            owner_refs: vec![],
            replicas: 2,
            containers: vec![],
            signals: PodSignals::default(),
            config_map_refs: vec![],
            secret_refs: vec![],
            pvc_refs: vec![],
            env_endpoints: vec![],
            manifest: None,
        };
        let mut db = a.clone();
        a.name = "web".into();
        db.name = "db".into();
        db.kind = "StatefulSet".into();
        db.pvc_refs.push("data".into());
        let raw = RawInventory {
            connection: "k3s".into(),
            discovered_at: "2026-07-11T00:00:00Z".into(),
            workloads: vec![a, db],
            ..Default::default()
        };
        InventorySnapshot::from_raw(raw)
    }

    #[test]
    fn test_report_builds_and_renders() {
        let report = build_report(&snap());
        assert_eq!(report.totals.applications, 1); // both grouped under "shop"
        let md = report.to_markdown();
        assert!(md.contains("Cloud Exit Assessment"));
        assert!(md.contains("Migration waves"));
        assert!(report.to_json().unwrap().contains("\"connection\""));
    }
}
