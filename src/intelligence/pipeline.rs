// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Intent → Infrastructure pipeline — outcome to deployable spec + placement.

use crate::engine::Engine;
use crate::intelligence::placement::{GlobalPlacementEngine, PlacementRecommendation};
use crate::spec::{IntentGoal, Workload};
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPipelineRequest {
    pub yaml: Option<String>,
    #[serde(default)]
    pub goals: Vec<String>,
    pub workload_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPipelineStep {
    pub phase: String,
    pub title: String,
    pub detail: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPipelineReport {
    pub generated_at: String,
    pub workload_name: String,
    pub intent_yaml: String,
    pub spec_yaml: String,
    pub recommended_runtime: String,
    pub confidence: f64,
    pub placement: Vec<PlacementRecommendation>,
    pub steps: Vec<IntentPipelineStep>,
}

pub async fn build_intent_pipeline(req: &IntentPipelineRequest) -> anyhow::Result<IntentPipelineReport> {
    let spec = if let Some(ref yaml) = req.yaml {
        serde_yaml::from_str(yaml)?
    } else {
        build_spec_from_goals(req)?
    };
    spec.validate()?;

    let engine = Engine::new();
    let recommended = engine.decide(&spec)?;
    let config = crate::config::Config::load();
    let intel = crate::intelligence::store::IntelligenceStore::load(
        &crate::intelligence::store::IntelligenceStore::default_path(),
    )
    .unwrap_or_default();
    let scoring_engine = crate::ai::scoring::ScoringEngine::new(config.engine)
        .with_history(intel.runtime_history_map());
    let scoring = scoring_engine.score(&spec);
    let confidence = scoring.confidence;

    let clusters = crate::kubecluster::list_clusters().await.unwrap_or_default();
    let placement = GlobalPlacementEngine::recommend(&spec, &clusters);
    let intent_yaml = intent_block_yaml(&spec);
    let spec_yaml = serde_yaml::to_string(&spec)?;

    let steps = vec![
        IntentPipelineStep {
            phase: "intent".into(),
            title: "Capture outcome".into(),
            detail: format!(
                "Goal: {} · workload {}",
                intent_goal_label(&spec.intent.as_ref().map(|i| i.goal.clone()).unwrap_or(IntentGoal::Balanced)),
                spec.metadata.name
            ),
            action: "Review intent block in spec".into(),
        },
        IntentPipelineStep {
            phase: "score".into(),
            title: "Score runtimes".into(),
            detail: format!(
                "Recommended {} with {:.0}% confidence",
                recommended,
                confidence * 100.0
            ),
            action: "Open Runtime Advisor for explainability".into(),
        },
        IntentPipelineStep {
            phase: "place".into(),
            title: "Place across fabric".into(),
            detail: placement
                .first()
                .map(|p| {
                    format!(
                        "Top placement: {} on {} (score {:.2})",
                        p.runtime,
                        p.cluster.as_deref().unwrap_or("local"),
                        p.score
                    )
                })
                .unwrap_or_else(|| "Local placement — no reachable clusters".into()),
            action: "Confirm cluster target before deploy".into(),
        },
        IntentPipelineStep {
            phase: "deploy".into(),
            title: "Deploy workload".into(),
            detail: "Validate policy, estimate cost, then apply spec".into(),
            action: format!("aether deploy --spec {}", spec.metadata.name),
        },
    ];

    Ok(IntentPipelineReport {
        generated_at: crate::resources::now_rfc3339(),
        workload_name: spec.metadata.name.clone(),
        intent_yaml,
        spec_yaml,
        recommended_runtime: format!("{recommended}"),
        confidence,
        placement,
        steps,
    })
}

pub fn build_spec_from_goals(req: &IntentPipelineRequest) -> anyhow::Result<Workload> {
    let name = req
        .workload_name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "intent-app".into());
    let goal = req
        .goals
        .first()
        .map(|g| goal_from_tag(g))
        .unwrap_or(IntentGoal::Balanced);

    let goal_label = intent_goal_label(&goal);

    let yaml = format!(
        r#"apiVersion: aether/v1
kind: Workload
metadata:
  name: {name}
  owner: platform
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 500m
  memory: 512Mi
  storage: 1Gi
intent:
  goal: {goal_label}
runtime:
  preferred: auto
  allow:
    - kube
    - container
"#
    );
    Ok(serde_yaml::from_str(&yaml)?)
}

fn goal_from_tag(tag: &str) -> IntentGoal {
    match tag.to_lowercase().as_str() {
        "low-latency" | "performance" | "high-latency" | "latency" => IntentGoal::LowLatency,
        "high-throughput" | "throughput" => IntentGoal::HighThroughput,
        "cost" | "cost-optimized" | "finops" => IntentGoal::CostOptimized,
        _ => IntentGoal::Balanced,
    }
}

fn intent_goal_label(goal: &IntentGoal) -> &'static str {
    match goal {
        IntentGoal::LowLatency => "low-latency",
        IntentGoal::HighThroughput => "high-throughput",
        IntentGoal::CostOptimized => "cost-optimized",
        IntentGoal::Balanced => "balanced",
    }
}

pub fn intent_block_yaml(spec: &Workload) -> String {
    if let Some(intent) = &spec.intent {
        serde_yaml::to_string(intent).unwrap_or_else(|_| format!("goal: {}", intent_goal_label(&intent.goal)))
    } else {
        "goal: balanced".into()
    }
}

/// Fleet-wide autonomous placement summary.
pub fn build_autonomous_placement(state_path: &Path) -> anyhow::Result<crate::intelligence::evolution::EvolutionStatus> {
    let store = StateStore::load(state_path)?;
    let pairs: Vec<(Workload, WorkloadState)> = store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect();
    let config = crate::config::Config::load();
    let policy = crate::intelligence::policy::AutonomyPolicy::from_config_and_workload(
        config.reconciliation.auto_reconcile,
        None,
    );
    Ok(crate::intelligence::evolution::EvolutionEngine::status_for_fleet(
        &pairs, &policy,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pipeline_from_goals_produces_steps() {
        let report = build_intent_pipeline(&IntentPipelineRequest {
            yaml: None,
            goals: vec!["cost".into()],
            workload_name: Some("shop".into()),
        })
        .await
        .unwrap();
        assert_eq!(report.workload_name, "shop");
        assert_eq!(report.steps.len(), 4);
        assert!(!report.spec_yaml.is_empty());
    }
}
