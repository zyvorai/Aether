// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Workload diagnosis — gather live cluster evidence for AI troubleshooting.

use crate::kubecluster::{
    health_summary, related_events, workload_detail, workload_logs, ClusterLogsRequest,
};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnoseRequest {
    pub workload: String,
    pub cluster: Option<String>,
    pub namespace: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnoseEvent {
    pub type_: String,
    pub reason: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnoseRecommendation {
    pub title: String,
    pub summary: String,
    pub action: String,
    /// When true, the dashboard can execute this fix via API without navigation.
    #[serde(default)]
    pub applyable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnoseResponse {
    pub workload: String,
    pub runtime: Option<String>,
    pub source: String,
    pub health_level: String,
    pub summary: String,
    pub ready_pods: usize,
    pub total_pods: usize,
    pub warning_events: usize,
    pub events: Vec<DiagnoseEvent>,
    pub log_excerpt: Option<String>,
    pub pods: Vec<PodEvidence>,
    pub recommendations: Vec<DiagnoseRecommendation>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodEvidence {
    pub name: String,
    pub phase: String,
    pub ready: usize,
    pub total_containers: usize,
    pub restarts: i32,
    pub node: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedTarget {
    display_name: String,
    source: String,
    runtime: Option<String>,
    cluster: Option<String>,
    namespace: Option<String>,
    kind: Option<String>,
    resource_name: String,
    state: Option<WorkloadState>,
}

pub async fn diagnose_workload(
    req: &DiagnoseRequest,
    store: &StateStore,
) -> Result<DiagnoseResponse> {
    let target = resolve_target(req, store).await?;
    let mut evidence = Vec::new();
    let mut recommendations = Vec::new();

    if let (Some(cluster), Some(namespace), Some(kind)) = (
        target.cluster.as_deref(),
        target.namespace.as_deref(),
        target.kind.as_deref(),
    ) {
        let logs_req = ClusterLogsRequest {
            cluster: cluster.to_string(),
            namespace: namespace.to_string(),
            kind: kind.to_string(),
            name: target.resource_name.clone(),
            ..Default::default()
        };

        let health = health_summary(&logs_req).await.unwrap_or_else(|_| {
            crate::kubecluster::ClusterHealthSummary {
                level: "unknown".into(),
                summary: "Could not load cluster health".into(),
                ready_pods: 0,
                total_pods: 0,
                warning_events: 0,
            }
        });
        evidence.push(format!(
            "Cluster health: {} — {}",
            health.level, health.summary
        ));

        let events: Vec<DiagnoseEvent> =
            related_events(cluster, namespace, kind, &target.resource_name)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|e| DiagnoseEvent {
                    type_: e.type_,
                    reason: e.reason,
                    message: e.message,
                    timestamp: e.timestamp,
                })
                .collect();
        if !events.is_empty() {
            evidence.push(format!(
                "Latest event: [{}] {} — {}",
                events[0].type_, events[0].reason, events[0].message
            ));
        }

        let detail = workload_detail(&logs_req).await.ok();
        let pods: Vec<PodEvidence> = detail
            .as_ref()
            .map(|d| {
                d.pods
                    .iter()
                    .map(|p| PodEvidence {
                        name: p.name.clone(),
                        phase: p.phase.clone(),
                        ready: p.ready,
                        total_containers: p.total_containers,
                        restarts: p.restarts,
                        node: p.node.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        for pod in &pods {
            if pod.restarts > 3 {
                recommendations.push(DiagnoseRecommendation {
                    title: format!("{} is restarting frequently", pod.name),
                    summary: format!(
                        "Pod restarted {} times. Memory limits or failing probes may be too aggressive.",
                        pod.restarts
                    ),
                    action: "restart".into(),
                    applyable: true,
                });
            }
            if pod.phase == "Pending" {
                recommendations.push(DiagnoseRecommendation {
                    title: format!("{} cannot be scheduled", pod.name),
                    summary: "No node may have enough CPU/memory, or taints may block placement."
                        .into(),
                    action: "view_nodes".into(),
                    applyable: false,
                });
            }
        }

        let log_excerpt = workload_logs(&logs_req).await.ok().map(|logs| {
            let lines: Vec<&str> = logs.lines().collect();
            if lines.len() <= 40 {
                logs
            } else {
                lines[lines.len() - 40..].join("\n")
            }
        });

        if let Some(ref logs) = log_excerpt {
            let lower = logs.to_lowercase();
            if lower.contains("crashloop") || lower.contains("error") || lower.contains("exception")
            {
                evidence.push("Logs contain error patterns".into());
                if lower.contains("crashloop") {
                    recommendations.push(DiagnoseRecommendation {
                        title: "CrashLoopBackOff detected".into(),
                        summary:
                            "Restart the workload after fixing config, or restart now to retry."
                                .into(),
                        action: "restart".into(),
                        applyable: true,
                    });
                }
            }
            if lower.contains("connection refused") || lower.contains("timeout") {
                recommendations.push(DiagnoseRecommendation {
                    title: "Connectivity issue detected in logs".into(),
                    summary: "The app may not reach its database, service, or ingress backend."
                        .into(),
                    action: "check_network".into(),
                    applyable: false,
                });
            }
            if lower.contains("imagepull") || lower.contains("errimagepull") {
                recommendations.push(DiagnoseRecommendation {
                    title: "Image pull failure".into(),
                    summary: "Verify image name, registry credentials, and network reachability."
                        .into(),
                    action: "fix_image".into(),
                    applyable: false,
                });
            }
        }

        for event in &events {
            if (event.reason.contains("Failed") || event.type_ == "Warning")
                && event.message.to_lowercase().contains("secret")
            {
                recommendations.push(DiagnoseRecommendation {
                    title: "Secret change may have caused failure".into(),
                    summary: event.message.clone(),
                    action: "rollback_secret".into(),
                    applyable: target.state.is_some(),
                });
            }
        }

        if health.level == "failing" && recommendations.is_empty() {
            recommendations.push(DiagnoseRecommendation {
                title: format!("{} is failing", target.display_name),
                summary: health.summary.clone(),
                action: "open_logs".into(),
                applyable: false,
            });
        }

        return Ok(DiagnoseResponse {
            workload: target.display_name,
            runtime: target.runtime,
            source: target.source,
            health_level: health.level,
            summary: health.summary,
            ready_pods: health.ready_pods,
            total_pods: health.total_pods,
            warning_events: health.warning_events,
            events: events.into_iter().take(15).collect(),
            log_excerpt,
            pods,
            recommendations,
            evidence,
        });
    }

    // Aether-managed workload (no live cluster coordinates)
    if let Some(ws) = target.state.as_ref() {
        let health =
            crate::health::HealthHistory::load(&crate::health::HealthHistory::default_path())
                .unwrap_or_default();
        let uptime = health.uptime_percent(&ws.name);
        let restarts = health.restart_count(&ws.name);
        evidence.push(format!("Uptime {:.1}%, restarts {restarts}", uptime));

        if let Ok(spec) = Workload::from_file(&ws.spec_path) {
            let report = crate::drift::DriftDetector::new().detect(&spec, ws);
            if report.has_drift {
                evidence.push(format!("Drift detected: {} item(s)", report.drifts.len()));
                recommendations.push(DiagnoseRecommendation {
                    title: "Configuration drift detected".into(),
                    summary: "Desired spec differs from running state.".into(),
                    action: "reconcile_drift".into(),
                    applyable: true,
                });
            }
        }

        if restarts > 2 {
            recommendations.push(DiagnoseRecommendation {
                title: format!("{} has restarted often", ws.name),
                summary: "Check logs and recent deployment changes.".into(),
                action: "restart".into(),
                applyable: true,
            });
        }

        return Ok(DiagnoseResponse {
            workload: ws.name.clone(),
            runtime: target.runtime,
            source: "aether".into(),
            health_level: if uptime > 95.0 {
                "healthy".into()
            } else if uptime > 80.0 {
                "degraded".into()
            } else {
                "failing".into()
            },
            summary: format!("Uptime {:.1}%, {restarts} restarts", uptime),
            ready_pods: 0,
            total_pods: 0,
            warning_events: 0,
            events: Vec::new(),
            log_excerpt: None,
            pods: Vec::new(),
            recommendations,
            evidence,
        });
    }

    anyhow::bail!("workload not found: {}", req.workload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetRootCauseEntry {
    pub workload: String,
    pub likely_cause: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub recommendation: String,
    pub health_level: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetRootCauseReport {
    pub generated_at: String,
    pub scanned: usize,
    pub diagnoses: Vec<FleetRootCauseEntry>,
}

/// Infer root cause label, confidence, and recommendation from a diagnosis report.
pub fn infer_root_cause(report: &DiagnoseResponse) -> (String, f64, String) {
    let log = report.log_excerpt.as_deref().unwrap_or("");
    let combined =
        format!("{} {} {}", report.summary, log, report.evidence.join(" ")).to_lowercase();

    if combined.contains("oom")
        || combined.contains("out of memory")
        || combined.contains("memory limit")
    {
        let rec = report
            .recommendations
            .iter()
            .find(|r| {
                r.title.to_lowercase().contains("memory")
                    || r.summary.to_lowercase().contains("memory")
            })
            .map(|r| r.summary.clone())
            .unwrap_or_else(|| "Increase memory limits or reduce workload memory footprint".into());
        return ("OOM".into(), 0.91, rec);
    }

    if report.pods.iter().any(|p| p.restarts >= 3) {
        let rec = report
            .recommendations
            .first()
            .map(|r| r.summary.clone())
            .unwrap_or_else(|| "Inspect crash logs and recent deployment changes".into());
        return ("Crash loop / instability".into(), 0.85, rec);
    }

    if combined.contains("imagepull") || combined.contains("errimagepull") {
        return (
            "Image pull failure".into(),
            0.88,
            "Verify image name, registry credentials, and network reachability".into(),
        );
    }

    if combined.contains("drift") {
        return (
            "Configuration drift".into(),
            0.82,
            "Reconcile desired spec with running state".into(),
        );
    }

    if let Some(rec) = report.recommendations.first() {
        return (rec.title.clone(), 0.72, rec.summary.clone());
    }

    (
        report.summary.clone(),
        0.55,
        "Review logs, events, and recent changes".into(),
    )
}

/// Batch-diagnose unhealthy workloads across the fleet (state-managed workloads).
pub async fn diagnose_fleet(store: &StateStore, max: usize) -> FleetRootCauseReport {
    let names: Vec<String> = store.list().iter().map(|ws| ws.name.clone()).collect();
    let mut diagnoses = Vec::new();

    for name in names.into_iter().take(max) {
        let req = DiagnoseRequest {
            workload: name.clone(),
            cluster: None,
            namespace: None,
            kind: None,
        };
        let Ok(report) = diagnose_workload(&req, store).await else {
            continue;
        };
        if report.health_level == "healthy" {
            continue;
        }
        let (likely_cause, confidence, recommendation) = infer_root_cause(&report);
        diagnoses.push(FleetRootCauseEntry {
            workload: report.workload.clone(),
            likely_cause,
            confidence,
            evidence: report.evidence.clone(),
            recommendation,
            health_level: report.health_level.clone(),
            summary: report.summary.clone(),
        });
    }

    diagnoses.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    FleetRootCauseReport {
        generated_at: crate::resources::now_rfc3339(),
        scanned: store.list().len(),
        diagnoses,
    }
}

async fn resolve_target(req: &DiagnoseRequest, store: &StateStore) -> Result<ResolvedTarget> {
    if let Some(cluster) = req.cluster.as_deref().filter(|c| !c.is_empty()) {
        let namespace = req
            .namespace
            .clone()
            .filter(|n| !n.is_empty())
            .context("namespace required with cluster")?;
        let kind = req
            .kind
            .clone()
            .filter(|k| !k.is_empty())
            .unwrap_or_else(|| "Deployment".into());
        let resource_name = short_name(&req.workload);
        return Ok(ResolvedTarget {
            display_name: req.workload.clone(),
            source: "cluster".into(),
            runtime: Some(if kind == "VirtualMachineInstance" {
                "KubeVirt".into()
            } else {
                "Kubernetes".into()
            }),
            cluster: Some(cluster.to_string()),
            namespace: Some(namespace),
            kind: Some(kind),
            resource_name,
            state: None,
        });
    }

    let parts: Vec<&str> = req.workload.split('/').collect();
    if parts.len() >= 3 {
        return Ok(ResolvedTarget {
            display_name: req.workload.clone(),
            source: "cluster".into(),
            runtime: Some("Kubernetes".into()),
            cluster: Some(parts[0].to_string()),
            namespace: Some(parts[1].to_string()),
            kind: req.kind.clone().or_else(|| Some("Deployment".into())),
            resource_name: parts[2].to_string(),
            state: None,
        });
    }

    if let Some(ws) = store.get(&req.workload) {
        return Ok(ResolvedTarget {
            display_name: ws.name.clone(),
            source: "aether".into(),
            runtime: Some(format!("{}", ws.runtime)),
            cluster: None,
            namespace: None,
            kind: None,
            resource_name: ws.name.clone(),
            state: Some(ws.clone()),
        });
    }

    let discovered = crate::kubecluster::list_workloads()
        .await
        .unwrap_or_default();
    let needle = req.workload.to_lowercase();
    if let Some(w) = discovered.iter().find(|w| {
        w.name.to_lowercase() == needle
            || format!("{}/{}/{}", w.cluster, w.namespace, w.name).to_lowercase() == needle
    }) {
        return Ok(ResolvedTarget {
            display_name: format!("{}/{}/{}", w.cluster, w.namespace, w.name),
            source: "cluster".into(),
            runtime: Some(if w.kind == "VirtualMachineInstance" {
                "KubeVirt".into()
            } else {
                "Kubernetes".into()
            }),
            cluster: Some(w.cluster.clone()),
            namespace: Some(w.namespace.clone()),
            kind: Some(w.kind.clone()),
            resource_name: w.name.clone(),
            state: None,
        });
    }

    anyhow::bail!("workload not found: {}", req.workload)
}

fn short_name(workload: &str) -> String {
    workload
        .split('/')
        .next_back()
        .unwrap_or(workload)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_root_cause_detects_oom() {
        let report = DiagnoseResponse {
            workload: "web".into(),
            runtime: Some("kube".into()),
            source: "aether".into(),
            health_level: "failing".into(),
            summary: "Pod OOMKilled".into(),
            ready_pods: 0,
            total_pods: 1,
            warning_events: 1,
            events: vec![],
            log_excerpt: Some("Out of memory".into()),
            pods: vec![],
            recommendations: vec![],
            evidence: vec!["Memory spike".into()],
        };
        let (cause, confidence, _) = infer_root_cause(&report);
        assert_eq!(cause, "OOM");
        assert!(confidence >= 0.9);
    }

    #[test]
    fn short_name_parses_cluster_path() {
        assert_eq!(short_name("prod/default/api"), "api");
        assert_eq!(short_name("api"), "api");
    }
}
