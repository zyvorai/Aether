// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Copilot tool registry — maps tool names to intelligence/API operations.

use crate::intelligence::context::build_context_snapshot;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::predict::FailurePredictor;
use crate::intelligence::security::SecurityEngine;
use crate::rbac::Role;
use crate::spec::Workload;
use crate::state::StateStore;
use crate::zyra::policy::{role_allows_tool, tool_risk};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub fn tools_openai_schema() -> serde_json::Value {
    json!([
        {"type":"function","function":{"name":"list_workloads","description":"List all workloads","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"context_snapshot","description":"Unified platform context","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"explain_health","description":"Explain workload health","parameters":{"type":"object","properties":{"workload":{"type":"string"}},"required":["workload"]}}},
        {"type":"function","function":{"name":"check_drift","description":"Check drift for workload","parameters":{"type":"object","properties":{"workload":{"type":"string"}}}}},
        {"type":"function","function":{"name":"recommend_runtime","description":"Score runtimes for YAML spec","parameters":{"type":"object","properties":{"yaml":{"type":"string"}}}}},
        {"type":"function","function":{"name":"cluster_summary","description":"List clusters","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"predictions","description":"Failure predictions","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"cost_summary","description":"Cost optimization recommendations","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"threats","description":"Security threat scan","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"query_metrics","description":"Prometheus instant query","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
        {"type":"function","function":{"name":"explain_attestation_failure","description":"Explain Ragnarok attestation failure for a VM/workload","parameters":{"type":"object","properties":{"vm_id":{"type":"string"}},"required":["vm_id"]}}},
        {"type":"function","function":{"name":"confidential_migrate_plan","description":"Plan confidential migration with TEE compatibility checks","parameters":{"type":"object","properties":{"workload":{"type":"string"}},"required":["workload"]}}},
        {"type":"function","function":{"name":"trust_score_fleet","description":"AI confidential fleet analysis with trust scores and risk findings","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"intelligence_place","description":"Global placement recommendation for a workload YAML spec","parameters":{"type":"object","properties":{"yaml":{"type":"string"}},"required":["yaml"]}}},
        {"type":"function","function":{"name":"diagnose_workload","description":"Diagnose workload with live cluster evidence: health, events, logs, pods, recommendations","parameters":{"type":"object","properties":{"workload":{"type":"string"},"cluster":{"type":"string"},"namespace":{"type":"string"},"kind":{"type":"string"}},"required":["workload"]}}},
        {"type":"function","function":{"name":"gitops_status","description":"GitOps reconciliation status","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"policy_violations","description":"Scan fleet for policy violations","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"ai_insights","description":"Combined AI insights: predictions, threats, cost, evolution","parameters":{"type":"object","properties":{}}}},
        {"type":"function","function":{"name":"restart_workload","description":"Restart a workload (requires approval)","parameters":{"type":"object","properties":{"workload":{"type":"string"}},"required":["workload"]}}},
        {"type":"function","function":{"name":"deploy_spec","description":"Deploy workload from YAML spec (requires approval)","parameters":{"type":"object","properties":{"yaml":{"type":"string"}},"required":["yaml"]}}},
        {"type":"function","function":{"name":"generate_artifact","description":"Generate Terraform, Ansible, or Kubernetes manifest","parameters":{"type":"object","properties":{"kind":{"type":"string"},"prompt":{"type":"string"}},"required":["kind","prompt"]}}},
        {"type":"function","function":{"name":"exec_command","description":"Execute sandboxed command (requires approval)","parameters":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}}},
        {"type":"function","function":{"name":"create_ticket","description":"Create incident ticket via webhook (requires approval)","parameters":{"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"}},"required":["title"]}}},
        {"type":"function","function":{"name":"create_pull_request","description":"Open a pull request (requires approval)","parameters":{"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"},"branch":{"type":"string"}},"required":["title","branch"]}}}
    ])
}

pub struct ToolContext {
    pub state: Arc<RwLock<StateStore>>,
    pub state_path: PathBuf,
    pub role: Role,
}

pub async fn execute_tool(
    ctx: &ToolContext,
    name: &str,
    args: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let risk = tool_risk(name);
    if !role_allows_tool(&ctx.role, risk) {
        anyhow::bail!("RBAC: role {:?} cannot invoke tool {name}", ctx.role);
    }

    match name {
        "list_workloads" => {
            let store = ctx.state.read().await;
            let list: Vec<_> = store
                .list()
                .iter()
                .map(|w| json!({"name": w.name, "runtime": format!("{}", w.runtime)}))
                .collect();
            Ok(json!({"workloads": list}))
        }
        "context_snapshot" => {
            let snap = build_context_snapshot(&ctx.state_path).await?;
            Ok(serde_json::to_value(snap)?)
        }
        "explain_health" => {
            let workload = args.get("workload").and_then(|v| v.as_str()).unwrap_or("");
            let store = ctx.state.read().await;
            let ws = store
                .get(workload)
                .ok_or_else(|| anyhow::anyhow!("workload not found"))?;
            let health =
                crate::health::HealthHistory::load(&crate::health::HealthHistory::default_path())
                    .unwrap_or_default();
            Ok(json!({
                "workload": workload,
                "runtime": format!("{}", ws.runtime),
                "uptime_pct": health.uptime_percent(workload),
                "restart_count": health.restart_count(workload),
            }))
        }
        "check_drift" => {
            let workload = args.get("workload").and_then(|v| v.as_str());
            let store = ctx.state.read().await;
            if let Some(name) = workload {
                let ws = store
                    .get(name)
                    .ok_or_else(|| anyhow::anyhow!("workload not found"))?;
                let spec = Workload::from_file(&ws.spec_path)?;
                let report = crate::drift::DriftDetector::new().detect(&spec, ws);
                return Ok(serde_json::to_value(report)?);
            }
            let mut drifted = Vec::new();
            for ws in store.list() {
                if let Ok(spec) = Workload::from_file(&ws.spec_path) {
                    let report = crate::drift::DriftDetector::new().detect(&spec, ws);
                    if report.has_drift {
                        drifted.push(json!({"workload": ws.name, "items": report.drifts.len()}));
                    }
                }
            }
            Ok(json!({"drifted": drifted}))
        }
        "recommend_runtime" => {
            let yaml = args.get("yaml").and_then(|v| v.as_str()).unwrap_or("");
            let spec: Workload = serde_yaml::from_str(yaml)?;
            let config = crate::config::Config::load();
            let intel = crate::intelligence::store::IntelligenceStore::load(
                &crate::intelligence::store::IntelligenceStore::default_path(),
            )
            .unwrap_or_default();
            let engine = crate::ai::scoring::ScoringEngine::new(config.engine)
                .with_history(intel.runtime_history_map());
            let result = engine.score(&spec);
            Ok(serde_json::to_value(result)?)
        }
        "cluster_summary" => {
            let clusters = crate::kubecluster::list_clusters()
                .await
                .unwrap_or_default();
            Ok(json!({"clusters": clusters}))
        }
        "predictions" => {
            let pairs = workload_pairs(&ctx.state).await;
            let report = FailurePredictor::predict_fleet(&pairs);
            Ok(serde_json::to_value(report)?)
        }
        "cost_summary" => {
            let pairs = workload_pairs(&ctx.state).await;
            let report = FinOpsEngine::optimize_fleet(&pairs);
            Ok(serde_json::to_value(report)?)
        }
        "threats" => {
            let pairs = workload_pairs(&ctx.state).await;
            let report = SecurityEngine::scan_fleet(&pairs);
            Ok(serde_json::to_value(report)?)
        }
        "query_metrics" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("query required"))?;
            let result = crate::observability::prometheus_instant_query(query).await?;
            Ok(result)
        }
        "explain_attestation_failure" => {
            let vm_id = args.get("vm_id").and_then(|v| v.as_str()).unwrap_or("");
            let dir = ctx
                .state_path
                .parent()
                .unwrap_or(&ctx.state_path)
                .to_path_buf();
            let svc = crate::ragnarok::AttestationService::new(dir.clone());
            let gk = crate::ragnarok::guestkit::GuestKitService::new(dir).summary(vm_id);
            let explain = svc.explain_with_guestkit(vm_id, gk)?;
            Ok(serde_json::to_value(explain)?)
        }
        "confidential_migrate_plan" => {
            let name = args.get("workload").and_then(|v| v.as_str()).unwrap_or("");
            let store = ctx.state.read().await;
            let ws = store
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("workload not found"))?;
            let spec = Workload::from_file(&ws.spec_path)?;
            let host = crate::ragnarok::probe_host_tee();
            let plan = crate::ragnarok::migration::plan_confidential_migration_tee(
                &spec,
                host.sev_snp,
                host.sev_snp,
                host.tdx,
                host.tdx,
            );
            Ok(serde_json::to_value(plan)?)
        }
        "trust_score_fleet" => {
            let dir = ctx
                .state_path
                .parent()
                .unwrap_or(&ctx.state_path)
                .to_path_buf();
            let pairs = workload_pairs(&ctx.state).await;
            let confidential: Vec<_> = pairs
                .iter()
                .filter(|(s, _)| s.confidential.as_ref().is_some_and(|c| c.enabled))
                .map(|(s, ws)| (ws.name.as_str(), s, ws.runtime.to_string()))
                .collect();
            let refs: Vec<(&str, &Workload, &str)> = confidential
                .iter()
                .map(|(n, s, r)| (*n, *s, r.as_str()))
                .collect();
            Ok(serde_json::to_value(
                crate::ragnarok::intelligence::analyze_fleet(&refs, &dir),
            )?)
        }
        "confidential_analyze" => {
            let name = args.get("workload").and_then(|v| v.as_str()).unwrap_or("");
            let store = ctx.state.read().await;
            let ws = store
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("workload not found"))?;
            let spec = Workload::from_file(&ws.spec_path)?;
            let dir = ctx
                .state_path
                .parent()
                .unwrap_or(&ctx.state_path)
                .to_path_buf();
            Ok(serde_json::to_value(
                crate::ragnarok::intelligence::analyze_workload(
                    &spec,
                    &ws.runtime.to_string(),
                    &dir,
                ),
            )?)
        }
        "intelligence_place" => {
            let yaml = args
                .get("yaml")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("yaml required"))?;
            let spec: Workload = serde_yaml::from_str(yaml)?;
            let clusters = crate::kubecluster::list_clusters()
                .await
                .unwrap_or_default();
            Ok(serde_json::to_value(
                crate::intelligence::placement::GlobalPlacementEngine::recommend(&spec, &clusters),
            )?)
        }
        "evolution_status" => {
            use crate::config::Config;
            use crate::intelligence::evolution::EvolutionEngine;
            use crate::intelligence::policy::AutonomyPolicy;
            let config = Config::load();
            let policy = AutonomyPolicy::from_config_and_workload(
                config.reconciliation.auto_reconcile,
                None,
            );
            let pairs = workload_pairs(&ctx.state).await;
            Ok(serde_json::to_value(EvolutionEngine::status_for_fleet(
                &pairs, &policy,
            ))?)
        }
        "diagnose_workload" => {
            let workload = args
                .get("workload")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("workload required"))?;
            let store = ctx.state.read().await;
            let req = crate::zyra::diagnose::DiagnoseRequest {
                workload: workload.into(),
                cluster: args
                    .get("cluster")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                namespace: args
                    .get("namespace")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                kind: args
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            };
            let report = crate::zyra::diagnose::diagnose_workload(&req, &store).await?;
            Ok(serde_json::to_value(report)?)
        }
        "gitops_status" => {
            let path = crate::resources::aether_path("gitops.json");
            if !path.exists() {
                return Ok(serde_json::json!({
                    "configured": false,
                    "hint": "GitOps not configured"
                }));
            }
            let data = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&data)?)
        }
        "policy_violations" => {
            let pairs = workload_pairs(&ctx.state).await;
            let engine = crate::policy::PolicyEngine::production();
            let mut violations = Vec::new();
            for (spec, ws) in &pairs {
                let result = engine.evaluate(spec);
                if !result.passed {
                    violations.push(serde_json::json!({
                        "workload": ws.name,
                        "violations": result.violations,
                    }));
                }
            }
            Ok(serde_json::json!({"violations": violations}))
        }
        "ai_insights" => {
            let pairs = workload_pairs(&ctx.state).await;
            let predictions = FailurePredictor::predict_fleet(&pairs);
            let threats = SecurityEngine::scan_fleet(&pairs);
            let cost = FinOpsEngine::optimize_fleet(&pairs);
            Ok(serde_json::json!({
                "predictions": predictions,
                "threats": threats,
                "cost": cost,
            }))
        }
        "restart_workload" => {
            let workload = args.get("workload").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({
                "status": "pending_approval",
                "action": "restart",
                "workload": workload,
                "hint": "Confirm to restart workload"
            }))
        }
        "deploy_spec" => {
            let yaml = args.get("yaml").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({
                "status": "pending_approval",
                "action": "deploy",
                "spec_preview": yaml.chars().take(200).collect::<String>(),
            }))
        }
        "generate_artifact" => {
            let kind = args
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("kubernetes");
            let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({
                "kind": kind,
                "artifact": format!("# Generated {kind}\n# {prompt}\napiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: zyra-generated\n"),
            }))
        }
        "exec_command" => {
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({
                "status": "pending_approval",
                "command": command,
                "hint": "Sandboxed execution requires confirmation"
            }))
        }
        "create_ticket" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Zyra incident");
            Ok(serde_json::json!({
                "status": "pending_approval",
                "ticket": title,
                "webhook": std::env::var("AETHER_TICKET_WEBHOOK").unwrap_or_else(|_| "not_configured".into()),
            }))
        }
        "create_pull_request" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            Ok(serde_json::json!({
                "status": "pending_approval",
                "pull_request": title,
                "branch": args.get("branch").and_then(|v| v.as_str()).unwrap_or("zyra/auto"),
            }))
        }
        other => anyhow::bail!("unknown tool: {other}"),
    }
}

async fn workload_pairs(
    state: &Arc<RwLock<StateStore>>,
) -> Vec<(Workload, crate::state::WorkloadState)> {
    let store = state.read().await;
    store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect()
}
