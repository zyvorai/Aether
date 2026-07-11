// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Observability summary and Prometheus proxy helpers.

use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;

use crate::kubecluster::cilium::CiliumStatusResponse;
use crate::kubecluster::ClusterMetricsSummary;
use crate::metrics;

#[derive(Debug, Clone, Serialize)]
pub struct ObservabilitySummary {
    pub api_http_requests_total: f64,
    pub migrations_total: f64,
    pub migration_rollbacks_total: f64,
    pub workloads_running: HashMap<String, f64>,
    pub cluster_metrics: Option<ClusterMetricsSummary>,
    pub cilium: Option<CiliumStatusResponse>,
    pub prometheus_configured: bool,
}

pub fn build_summary(
    cluster_metrics: Option<ClusterMetricsSummary>,
    cilium: Option<CiliumStatusResponse>,
) -> ObservabilitySummary {
    let text = metrics::gather();
    ObservabilitySummary {
        api_http_requests_total: sum_counter_metric(&text, "aether_api_http_requests_total"),
        migrations_total: sum_counter_metric(&text, "aether_migrations_total"),
        migration_rollbacks_total: sum_counter_metric(&text, "aether_migration_rollbacks_total"),
        workloads_running: gauge_values_by_label(&text, "aether_workload_running", "runtime"),
        cluster_metrics,
        cilium,
        prometheus_configured: std::env::var("AETHER_PROMETHEUS_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .is_some(),
    }
}

fn sum_counter_metric(text: &str, metric_name: &str) -> f64 {
    text.lines()
        .filter(|line| line.starts_with(metric_name) && !line.starts_with('#'))
        .filter_map(|line| line.split_whitespace().last()?.parse::<f64>().ok())
        .sum()
}

fn gauge_values_by_label(text: &str, metric_name: &str, label: &str) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    for line in text.lines() {
        if line.starts_with('#') || !line.contains(metric_name) {
            continue;
        }
        if !line.starts_with(metric_name) {
            continue;
        }
        let value = line
            .split_whitespace()
            .last()
            .and_then(|v| v.parse::<f64>().ok());
        let Some(value) = value else { continue };
        if let Some(start) = line.find(&format!("{label}=\"")) {
            let rest = &line[start + label.len() + 2..];
            if let Some(end) = rest.find('"') {
                out.insert(rest[..end].to_string(), value);
            }
        } else if !line.contains('{') {
            out.insert("total".to_string(), value);
        }
    }
    out
}

pub fn query_allowed(promql: &str) -> bool {
    if crate::intelligence::metrics::query_allowed_extended(promql) {
        return true;
    }
    let trimmed = promql.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_lowercase();
    lower.starts_with("aether_")
        || lower.starts_with("kube_")
        || lower.starts_with("sum(aether_")
        || lower.starts_with("sum(kube_")
        || lower.starts_with("rate(aether_")
        || lower.starts_with("rate(kube_")
}

pub async fn prometheus_instant_query(query: &str) -> Result<serde_json::Value> {
    if !query_allowed(query) {
        anyhow::bail!("query must use aether_ or kube_ metric prefixes");
    }
    let base =
        std::env::var("AETHER_PROMETHEUS_URL").context("AETHER_PROMETHEUS_URL is not set")?;
    let base = base.trim_end_matches('/');
    let url = format!("{base}/api/v1/query");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let response = client
        .get(&url)
        .query(&[("query", query)])
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}

// ---------------------------------------------------------------------------
// Distributed tracing (OpenTelemetry / OTLP)
// ---------------------------------------------------------------------------

/// Build the OpenTelemetry tracing layer when `AETHER_OTLP_ENDPOINT` is set.
///
/// Opt-in: when the env var points at an OTLP/HTTP collector
/// (e.g. `http://localhost:4318`), all `tracing` spans are exported via a batch
/// OTLP exporter and the W3C trace-context propagator is installed so trace IDs
/// flow across service boundaries. When unset, returns `None` and tracing runs
/// with no exporter overhead.
pub fn otel_trace_layer<S>() -> Option<Box<dyn tracing_subscriber::Layer<S> + Send + Sync>>
where
    S: tracing::Subscriber
        + for<'a> tracing_subscriber::registry::LookupSpan<'a>
        + Send
        + Sync
        + 'static,
{
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig as _;
    use tracing_subscriber::Layer as _;

    let endpoint = std::env::var("AETHER_OTLP_ENDPOINT")
        .ok()
        .filter(|s| !s.trim().is_empty())?;

    // Install the W3C trace-context propagator for cross-service correlation.
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint.clone())
        .build()
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("OTLP exporter init failed ({endpoint}): {e}");
            return None;
        }
    };

    let resource = opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        "aether",
    )]);
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("aether");
    opentelemetry::global::set_tracer_provider(provider);

    Some(tracing_opentelemetry::layer().with_tracer(tracer).boxed())
}

/// Flush any buffered spans and shut down the global tracer provider.
/// Safe to call even when tracing was never initialised.
pub fn otel_shutdown() {
    opentelemetry::global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_counter_metric() {
        let text = "# HELP aether_migrations_total x\naether_migrations_total{status=\"ok\"} 3\naether_migrations_total{status=\"fail\"} 1\n";
        assert_eq!(sum_counter_metric(text, "aether_migrations_total"), 4.0);
    }

    #[test]
    fn test_query_allowed() {
        assert!(query_allowed("aether_workload_running"));
        assert!(query_allowed("rate(aether_api_http_requests_total[5m])"));
        assert!(query_allowed("node_cpu_seconds_total"));
        assert!(query_allowed("container_memory_working_set_bytes"));
        assert!(!query_allowed("up"));
    }
}
