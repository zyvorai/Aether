// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Parse simplified dashboard/CLI workload YAML into [`crate::spec::Workload`].
//!
//! Accepts legacy one-line runtime strings (`runtime: podman`) and flat `name`/`image` fields
//! in addition to full `apiVersion: aether/v1` documents.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::spec::{
    BuildSpec, HealthProbe, HealthSpec, IntentGoal, IntentSpec, Metadata, ProbeType,
    ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType, ScalingSpec, Workload,
};

#[derive(Debug, Deserialize)]
struct LegacyWorkloadYaml {
    name: String,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    runtime: Option<String>,
    #[serde(default)]
    replicas: Option<u32>,
    #[serde(default)]
    resources: Option<LegacyResources>,
    #[serde(default)]
    intent: Option<String>,
    #[serde(default)]
    health_check: Option<LegacyHealthCheck>,
}

#[derive(Debug, Deserialize)]
struct LegacyResources {
    #[serde(default)]
    cpu: Option<String>,
    #[serde(default)]
    memory: Option<String>,
    #[serde(default)]
    storage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LegacyHealthCheck {
    #[serde(default)]
    http_get: Option<LegacyHttpGet>,
}

#[derive(Debug, Deserialize)]
struct LegacyHttpGet {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    port: Option<u16>,
}

/// Try full v1 [`Workload`] YAML, then legacy flat format.
pub fn parse_workload_yaml(yaml: &str) -> Result<Workload> {
    if let Ok(w) = serde_yaml::from_str::<Workload>(yaml) {
        return Ok(w);
    }
    parse_legacy_workload_yaml(yaml)
}

fn parse_legacy_workload_yaml(yaml: &str) -> Result<Workload> {
    let legacy: LegacyWorkloadYaml =
        serde_yaml::from_str(yaml).context("legacy workload YAML parse error")?;
  if legacy.name.trim().is_empty() {
        anyhow::bail!("name is required");
    }

    let (registry, tag, repo) = parse_image_ref(legacy.image.as_deref().unwrap_or(&legacy.name));
    let (preferred, allow) = runtime_from_legacy(legacy.runtime.as_deref());

    let metadata_name = if legacy.image.is_some() {
        repo.clone()
    } else {
        legacy.name.clone()
    };
    let mut labels = HashMap::new();
    if legacy.image.is_some() && legacy.name != metadata_name {
        labels.insert("aether.io/display-name".to_string(), legacy.name.clone());
    }

    let cpu = legacy
        .resources
        .as_ref()
        .and_then(|r| r.cpu.clone())
        .unwrap_or_else(|| "500m".to_string());
    let memory = legacy
        .resources
        .as_ref()
        .and_then(|r| r.memory.clone())
        .unwrap_or_else(|| "512Mi".to_string());
    let storage = legacy
        .resources
        .as_ref()
        .and_then(|r| r.storage.clone())
        .unwrap_or_else(|| "1Gi".to_string());

    let workload = Workload {
        api_version: "aether/v1".to_string(),
        kind: "Workload".to_string(),
        metadata: Metadata {
            name: metadata_name,
            owner: "dashboard".to_string(),
            project: "default".to_string(),
            labels,
            annotations: HashMap::new(),
        },
        build: BuildSpec {
            context: PathBuf::from("."),
            dockerfile: PathBuf::from("Dockerfile"),
            registry,
            build_args: HashMap::new(),
            tag: Some(tag),
            push: false,
        },
        requirements: ResourceRequirements {
            cpu,
            memory,
            storage,
            gpu: None,
            cpu_request: None,
            memory_request: None,
        },
        runtime: RuntimeSpec { preferred, allow },
        network: Default::default(),
        persistence: Default::default(),
        health: legacy.health_check.as_ref().map(|h| {
            let path = h
                .http_get
                .as_ref()
                .and_then(|g| g.path.clone())
                .unwrap_or_else(|| "/health".to_string());
            let port = h.http_get.as_ref().and_then(|g| g.port).unwrap_or(80);
            HealthSpec {
                liveness: None,
                readiness: Some(HealthProbe {
                    probe_type: ProbeType::HttpGet { path, port },
                    initial_delay_seconds: 5,
                    period_seconds: 10,
                }),
                startup: None,
            }
        }),
        config: None,
        ingress: None,
        scaling: legacy.replicas.filter(|&n| n > 1).map(|n| ScalingSpec {
            enabled: true,
            min_replicas: n,
            max_replicas: n,
            metrics: vec![],
            behavior: None,
        }),
        mesh: None,
        intent: legacy.intent.as_deref().map(intent_from_legacy),
        autonomy: None,
        confidential: None,
        schedule: None,
        kubernetes: None,
    };

    workload.validate()?;
    Ok(workload)
}

fn parse_image_ref(image: &str) -> (String, String, String) {
    let trimmed = image.trim();
    let mut tag = "latest".to_string();
    let mut rest = trimmed.to_string();
    if let Some((base, t)) = trimmed.rsplit_once(':') {
        if !base.contains('/') || base.contains('.') || base.starts_with("localhost") {
            rest = base.to_string();
            tag = t.to_string();
        }
    }

    let parts: Vec<&str> = rest.split('/').collect();
    let (registry, repo) = match parts.len() {
        1 => ("docker.io/library".to_string(), parts[0].to_string()),
        2 if parts[0].contains('.') || parts[0].contains(':') || parts[0] == "localhost" => {
            (parts[0].to_string(), parts[1].to_string())
        }
        _ => {
            let repo = parts.last().unwrap().to_string();
            let registry = parts[..parts.len() - 1].join("/");
            (registry, repo)
        }
    };
    (registry, tag, repo)
}

fn runtime_from_legacy(raw: Option<&str>) -> (RuntimePreference, Vec<RuntimeType>) {
    match raw.unwrap_or("auto").to_lowercase().as_str() {
        "podman" | "docker" | "container" => {
            (RuntimePreference::Container, vec![RuntimeType::Container])
        }
        "kubernetes" | "kube" | "k8s" => (RuntimePreference::Kube, vec![RuntimeType::Kube]),
        "kubevirt" | "vm" => (RuntimePreference::Kubevirt, vec![RuntimeType::Kubevirt]),
        "metal3" | "metal" | "baremetal" => (RuntimePreference::Metal, vec![RuntimeType::Metal]),
        _ => (
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
                RuntimeType::Metal,
            ],
        ),
    }
}

fn intent_from_legacy(raw: &str) -> IntentSpec {
    let goal = match raw.to_lowercase().as_str() {
        "low-latency" | "low_latency" => IntentGoal::LowLatency,
        "high-throughput" | "high_throughput" => IntentGoal::HighThroughput,
        "cost-optimized" | "cost_optimized" | "cost" => IntentGoal::CostOptimized,
        _ => IntentGoal::Balanced,
    };
    IntentSpec {
        goal,
        sla: None,
        budget: None,
        resilience: None,
        compliance: None,
        trust: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_runtime_string_parses() {
        let yaml = r#"name: demo
image: nginx:latest
runtime: kubernetes
replicas: 2
resources:
  cpu: 500m
  memory: 512Mi
"#;
        let w = parse_workload_yaml(yaml).expect("legacy yaml");
        assert_eq!(w.metadata.name, "nginx");
        assert_eq!(w.runtime.preferred, RuntimePreference::Kube);
        assert_eq!(w.build.registry, "docker.io/library");
        assert_eq!(w.build.tag.as_deref(), Some("latest"));
    }
}
