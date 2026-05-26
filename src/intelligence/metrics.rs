//! Prometheus metrics helpers for intelligence modules.

use crate::ai::scaling::TimeSeries;
use anyhow::{Context, Result};

pub fn query_allowed_extended(promql: &str) -> bool {
    let trimmed = promql.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("aether_")
        || lower.starts_with("kube_")
        || lower.starts_with("sum(aether_")
        || lower.starts_with("sum(kube_")
        || lower.starts_with("rate(aether_")
        || lower.starts_with("rate(kube_")
    {
        return true;
    }
    lower.starts_with("container_")
        || lower.starts_with("node_")
        || lower.starts_with("kubelet_")
        || lower.starts_with("rate(container_")
        || lower.starts_with("rate(node_")
}

pub async fn prometheus_range_query(query: &str, range_secs: u64, step_secs: u64) -> Result<serde_json::Value> {
    if !query_allowed_extended(query) {
        anyhow::bail!("query must use allowed metric prefixes (aether_, kube_, container_, node_, kubelet_)");
    }
    let base = std::env::var("AETHER_PROMETHEUS_URL")
        .context("AETHER_PROMETHEUS_URL is not set")?;
    let base = base.trim_end_matches('/');
    let end = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let start = end.saturating_sub(range_secs);
    let url = format!("{base}/api/v1/query_range");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let response = client
        .get(&url)
        .query(&[
            ("query", query),
            ("start", &start.to_string()),
            ("end", &end.to_string()),
            ("step", &step_secs.to_string()),
        ])
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}

pub fn time_series_from_range(json: &serde_json::Value, name: &str, unit: &str) -> TimeSeries {
    let mut series = TimeSeries::new(name, unit);
    let Some(results) = json
        .pointer("/data/result")
        .and_then(|v| v.as_array())
    else {
        return series;
    };
    if let Some(first) = results.first() {
        if let Some(values) = first.get("values").and_then(|v| v.as_array()) {
            for pair in values {
                if let Some(arr) = pair.as_array() {
                    if arr.len() >= 2 {
                        let ts = arr[0].as_f64().unwrap_or(0.0);
                        let val = arr[1]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .or_else(|| arr[1].as_f64())
                            .unwrap_or(0.0);
                        series.add(ts, val);
                    }
                }
            }
        }
    }
    series
}

pub async fn fetch_fleet_utilization_series() -> (TimeSeries, TimeSeries) {
    let mut cpu = TimeSeries::new("cpu_utilization", "ratio");
    let mut mem = TimeSeries::new("memory_utilization", "ratio");

    if std::env::var("AETHER_PROMETHEUS_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        synthetic_series(&mut cpu, &mut mem);
        return (cpu, mem);
    }

    let cpu_q = "avg(rate(container_cpu_usage_seconds_total[5m]))";
    let mem_q = "avg(container_memory_working_set_bytes) / avg(container_spec_memory_limit_bytes)";

    match prometheus_range_query(cpu_q, 3600, 60).await {
        Ok(json) => cpu = time_series_from_range(&json, "cpu_utilization", "ratio"),
        Err(e) => {
            tracing::debug!("prometheus cpu range query failed: {e}");
            synthetic_series(&mut cpu, &mut mem);
            return (cpu, mem);
        }
    }

    match prometheus_range_query(mem_q, 3600, 60).await {
        Ok(json) => mem = time_series_from_range(&json, "memory_utilization", "ratio"),
        Err(e) => {
            tracing::debug!("prometheus mem range query failed: {e}");
            if cpu.points.is_empty() {
                synthetic_series(&mut cpu, &mut mem);
            }
        }
    }

    if cpu.points.is_empty() {
        synthetic_series(&mut cpu, &mut mem);
    }

    (cpu, mem)
}

fn synthetic_series(cpu: &mut TimeSeries, mem: &mut TimeSeries) {
    let base_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        - 3600.0;
    for i in 0..60 {
        let t = base_time + (i as f64 * 60.0);
        cpu.add(t, 0.45 + (i as f64 * 0.005) + ((i as f64 * 0.1).sin() * 0.05));
        mem.add(t, 0.55 + (i as f64 * 0.002));
    }
}
