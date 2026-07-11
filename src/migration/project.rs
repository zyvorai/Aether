// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Declarative migration artifacts (`apiVersion: aether.zyvor.dev/v1alpha1`):
//! `MigrationProject`, `MigrationPlan`, and `Assessment`. These are the
//! kind-dispatched YAML types the platform vision describes; `plan create`
//! generates a `MigrationPlan` from a portability assessment.

use crate::assessment::PortabilityAssessment;
use serde::{Deserialize, Serialize};

pub const API_VERSION: &str = "aether.zyvor.dev/v1alpha1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub labels: std::collections::BTreeMap<String, String>,
}

// ── MigrationProject ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProject {
    pub api_version: String,
    pub kind: String,
    pub metadata: ObjectMeta,
    pub spec: ProjectSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSpec {
    pub source: EndpointRef,
    pub target: EndpointRef,
    #[serde(default)]
    pub objectives: Objectives,
    #[serde(default)]
    pub approvals: Approvals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointRef {
    #[serde(rename = "type")]
    pub type_: String,
    pub connection_ref: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Objectives {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_downtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_rpo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retain_source_for: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approvals {
    #[serde(default)]
    pub cutover: ApprovalGate,
    #[serde(default)]
    pub rollback: ApprovalGate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApprovalGate {
    #[serde(default)]
    pub required: bool,
}

// ── MigrationPlan ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPlan {
    pub api_version: String,
    pub kind: String,
    pub metadata: ObjectMeta,
    pub spec: PlanSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSpec {
    pub source: PlanEndpoint,
    pub target: PlanEndpoint,
    pub strategy: String,
    pub class: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remediations: Vec<String>,
    pub rollback: RollbackSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEndpoint {
    pub connection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackSpec {
    pub enabled: bool,
    pub observation_window: String,
}

impl MigrationPlan {
    /// Generate a plan from a portability assessment.
    pub fn from_assessment(
        assessment: &PortabilityAssessment,
        source_conn: &str,
        target_conn: &str,
        strategy_override: Option<&str>,
    ) -> Self {
        let strategy = strategy_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| assessment.recommended_strategy.to_string());
        MigrationPlan {
            api_version: API_VERSION.to_string(),
            kind: "MigrationPlan".to_string(),
            metadata: ObjectMeta {
                name: assessment.application.clone(),
                namespace: Some(assessment.namespace.clone()),
                labels: Default::default(),
            },
            spec: PlanSpec {
                source: PlanEndpoint {
                    connection: source_conn.to_string(),
                    namespace: Some(assessment.namespace.clone()),
                },
                target: PlanEndpoint {
                    connection: target_conn.to_string(),
                    namespace: Some(assessment.namespace.clone()),
                },
                strategy,
                class: assessment.class.to_string(),
                blockers: assessment.blockers.clone(),
                remediations: assessment.remediations.clone(),
                rollback: RollbackSpec {
                    enabled: true,
                    observation_window: "2h".to_string(),
                },
            },
        }
    }

    pub fn to_yaml(&self) -> anyhow::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }
}

// ── Kind-dispatch envelope ──────────────────────────────────────────────────

/// A parsed Aether YAML document, dispatched by `kind`. `Workload` documents
/// (`apiVersion: aether/v1`) continue to load via `Workload::from_file`.
#[derive(Debug, Clone)]
pub enum AetherResource {
    MigrationProject(MigrationProject),
    MigrationPlan(MigrationPlan),
}

/// Parse an `aether.zyvor.dev/v1alpha1` document by peeking its `kind`.
pub fn parse_resource(yaml: &str) -> anyhow::Result<AetherResource> {
    #[derive(Deserialize)]
    struct Envelope {
        kind: String,
    }
    let env: Envelope = serde_yaml::from_str(yaml)?;
    match env.kind.as_str() {
        "MigrationProject" => Ok(AetherResource::MigrationProject(serde_yaml::from_str(yaml)?)),
        "MigrationPlan" => Ok(AetherResource::MigrationPlan(serde_yaml::from_str(yaml)?)),
        other => anyhow::bail!("unknown aether.zyvor.dev kind '{}'", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assessment::{Complexity, PortabilityAssessment};
    use crate::inventory::application::MigrationClass;

    fn sample_assessment() -> PortabilityAssessment {
        PortabilityAssessment {
            application: "payments".into(),
            namespace: "prod".into(),
            class: MigrationClass::StatefulNative,
            score: 80,
            complexity: Complexity::Medium,
            downtime_class: "< 10 minutes".into(),
            recommended_target: crate::runtime::RuntimeKind::Kubernetes,
            recommended_strategy: crate::migration::MigrationStrategy::BlueGreen,
            risk: crate::ai::migration::RiskLevel::Medium,
            estimated_downtime_secs: 300,
            monthly_cost_usd: Some(120.0),
            dependency_completeness: 100,
            blockers: vec![],
            warnings: vec![],
            remediations: vec![],
            compatibility: None,
        }
    }

    #[test]
    fn test_plan_from_assessment_roundtrips() {
        let plan = MigrationPlan::from_assessment(&sample_assessment(), "src", "dst", None);
        let yaml = plan.to_yaml().unwrap();
        assert!(yaml.contains("apiVersion: aether.zyvor.dev/v1alpha1"));
        assert!(yaml.contains("kind: MigrationPlan"));
        assert!(yaml.contains("strategy: blue-green"));

        // Re-parses via the kind-dispatch envelope.
        match parse_resource(&yaml).unwrap() {
            AetherResource::MigrationPlan(p) => {
                assert_eq!(p.metadata.name, "payments");
                assert_eq!(p.spec.source.connection, "src");
            }
            _ => panic!("wrong kind"),
        }
    }

    #[test]
    fn test_migration_project_parse() {
        let yaml = r#"
apiVersion: aether.zyvor.dev/v1alpha1
kind: MigrationProject
metadata:
  name: cloud-exit
spec:
  source: { type: eks, connectionRef: prod-eks }
  target: { type: kubernetes, connectionRef: zyvor }
  objectives: { maxDowntime: 10m, targetRPO: 1m }
  approvals: { cutover: { required: true } }
"#;
        match parse_resource(yaml).unwrap() {
            AetherResource::MigrationProject(p) => {
                assert_eq!(p.spec.source.connection_ref, "prod-eks");
                assert!(p.spec.approvals.cutover.required);
            }
            _ => panic!("wrong kind"),
        }
    }
}
