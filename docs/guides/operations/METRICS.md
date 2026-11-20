---
hero:
  eyebrow: GUIDES
  title: Prometheus Metrics
  tone: violet
---

Aether provides comprehensive Prometheus metrics for monitoring deployments, migrations, and system health.

## Overview

Metrics are automatically collected for:
- **Workload Operations**: Builds, deployments, deletions
- **Runtime Distribution**: Track workloads across runtimes
- **Migrations**: Success rates, duration, rollbacks
- **CLI Usage**: Command execution and timing
- **System Health**: Version info, runtime availability

## Exporting Metrics

### Command Line

Export metrics in Prometheus text format:

```bash
aether metrics
```

Output:
```
# Aether Metrics
# Updated: 2026-02-06T00:22:48.627648142+00:00

# HELP aether_aether_build_info Aether version and build information
# TYPE aether_aether_build_info counter
aether_aether_build_info{version="0.1.0"} 1

# HELP aether_workload_deployments_total Total number of workload deployments
# TYPE aether_workload_deployments_total counter
aether_workload_deployments_total{runtime="kubernetes",status="success"} 5
aether_workload_deployments_total{runtime="podman",status="success"} 3
...
```

### Save to File

```bash
aether metrics > /var/lib/aether/metrics.prom
```

### Integration with Prometheus

#### Node Exporter Textfile Collector

Use Prometheus Node Exporter's textfile collector:

```bash
#!/bin/bash
# /usr/local/bin/aether-metrics-exporter.sh

METRICS_DIR="/var/lib/node_exporter/textfile_collector"
aether metrics > "${METRICS_DIR}/aether.prom.$$"
mv "${METRICS_DIR}/aether.prom.$$" "${METRICS_DIR}/aether.prom"
```

Add to cron:
```
*/5 * * * * /usr/local/bin/aether-metrics-exporter.sh
```

#### Prometheus Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']
```

## Available Metrics

### Workload Metrics

#### `aether_workload_builds_total`
**Type:** Counter
**Labels:** `runtime`, `status`
**Description:** Total number of workload builds

Example:
```
aether_workload_builds_total{runtime="kubernetes",status="success"} 42
aether_workload_builds_total{runtime="podman",status="failure"} 2
```

#### `aether_workload_deployments_total`
**Type:** Counter
**Labels:** `runtime`, `status`
**Description:** Total number of workload deployments

Example:
```
aether_workload_deployments_total{runtime="kubernetes",status="success"} 38
```

#### `aether_workload_running`
**Type:** Gauge
**Labels:** `runtime`
**Description:** Number of currently running workloads

Example:
```
aether_workload_running{runtime="kubernetes"} 15
aether_workload_running{runtime="podman"} 3
```

#### `aether_workload_state`
**Type:** Gauge
**Labels:** `runtime`, `state`
**Description:** Workload state distribution

Example:
```
aether_workload_state{runtime="kubernetes",state="running"} 15
aether_workload_state{runtime="kubernetes",state="pending"} 2
```

### Migration Metrics

#### `aether_migrations_total`
**Type:** Counter
**Labels:** `source_runtime`, `target_runtime`, `strategy`, `status`
**Description:** Total number of migrations

Example:
```
aether_migrations_total{source_runtime="podman",target_runtime="kubernetes",strategy="blue-green",status="success"} 5
aether_migrations_total{source_runtime="kubernetes",target_runtime="kubevirt",strategy="immediate",status="failure"} 1
```

#### `aether_migration_duration_seconds`
**Type:** Histogram
**Labels:** `source_runtime`, `target_runtime`, `strategy`
**Description:** Migration duration in seconds
**Buckets:** 1, 5, 10, 30, 60, 120, 300, 600

Example:
```
aether_migration_duration_seconds_bucket{source_runtime="podman",target_runtime="kubernetes",strategy="blue-green",le="30"} 3
aether_migration_duration_seconds_sum{source_runtime="podman",target_runtime="kubernetes",strategy="blue-green"} 87.5
aether_migration_duration_seconds_count{source_runtime="podman",target_runtime="kubernetes",strategy="blue-green"} 5
```

#### `aether_migration_rollbacks_total`
**Type:** Counter
**Labels:** `source_runtime`, `target_runtime`, `strategy`
**Description:** Total number of migration rollbacks

Example:
```
aether_migration_rollbacks_total{source_runtime="kubernetes",target_runtime="kubevirt",strategy="blue-green"} 1
```

### Runtime Metrics

#### `aether_runtime_available`
**Type:** Gauge
**Labels:** `runtime`
**Description:** Runtime availability (1=available, 0=unavailable)

Example:
```
aether_runtime_available{runtime="kubernetes"} 1
aether_runtime_available{runtime="kubevirt"} 0
```

#### `aether_runtime_decision_seconds`
**Type:** Histogram
**Description:** Runtime decision time in seconds
**Buckets:** 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0

Example:
```
aether_runtime_decision_seconds_bucket{le="0.01"} 45
aether_runtime_decision_seconds_sum 0.234
aether_runtime_decision_seconds_count 50
```

### System Metrics

#### `aether_build_info`
**Type:** Counter
**Labels:** `version`
**Description:** Aether version and build information

Example:
```
aether_aether_build_info{version="0.1.0"} 1
```

#### `aether_cli_commands_total`
**Type:** Counter
**Labels:** `command`
**Description:** Total CLI commands executed

Example:
```
aether_cli_commands_total{command="deploy"} 38
aether_cli_commands_total{command="migrate"} 5
aether_cli_commands_total{command="list"} 12
```

#### `aether_command_duration_seconds`
**Type:** Histogram
**Labels:** `command`
**Description:** Command execution duration in seconds
**Buckets:** 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0

Example:
```
aether_command_duration_seconds_bucket{command="deploy",le="5"} 30
aether_command_duration_seconds_sum{command="deploy"} 125.3
aether_command_duration_seconds_count{command="deploy"} 38
```

## Example PromQL Queries

### Deployment Success Rate

```promql
rate(aether_workload_deployments_total{status="success"}[5m])
/
rate(aether_workload_deployments_total[5m])
```

### Migration Success Rate by Strategy

```promql
sum by (strategy) (
  rate(aether_migrations_total{status="success"}[5m])
)
/
sum by (strategy) (
  rate(aether_migrations_total[5m])
)
```

### Average Migration Duration

```promql
rate(aether_migration_duration_seconds_sum[5m])
/
rate(aether_migration_duration_seconds_count[5m])
```

### Workload Distribution by Runtime

```promql
sum by (runtime) (aether_workload_running)
```

### Command Latency (95th percentile)

```promql
histogram_quantile(0.95,
  rate(aether_command_duration_seconds_bucket[5m])
)
```

### Build Failure Rate

```promql
rate(aether_workload_builds_total{status="failure"}[5m])
/
rate(aether_workload_builds_total[5m])
```

## Grafana Dashboard

### Sample Dashboard JSON

Create a Grafana dashboard with these panels:

```json
{
  "dashboard": {
    "title": "Aether Monitoring",
    "panels": [
      {
        "title": "Workloads by Runtime",
        "targets": [
          {
            "expr": "sum by (runtime) (aether_workload_running)"
          }
        ],
        "type": "piechart"
      },
      {
        "title": "Deployment Success Rate",
        "targets": [
          {
            "expr": "rate(aether_workload_deployments_total{status=\"success\"}[5m]) / rate(aether_workload_deployments_total[5m])"
          }
        ],
        "type": "graph"
      },
      {
        "title": "Migration Duration",
        "targets": [
          {
            "expr": "rate(aether_migration_duration_seconds_sum[5m]) / rate(aether_migration_duration_seconds_count[5m])"
          }
        ],
        "type": "graph"
      }
    ]
  }
}
```

## Alerting Rules

### Sample Prometheus Alerts

```yaml
groups:
  - name: aether
    interval: 30s
    rules:
      - alert: HighDeploymentFailureRate
        expr: |
          (
            rate(aether_workload_deployments_total{status="failure"}[5m])
            /
            rate(aether_workload_deployments_total[5m])
          ) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High deployment failure rate"
          description: "Deployment failure rate is {{ $value | humanizePercentage }}"

      - alert: MigrationFailed
        expr: increase(aether_migrations_total{status="failure"}[5m]) > 0
        labels:
          severity: critical
        annotations:
          summary: "Migration failed"
          description: "Migration from {{ $labels.source_runtime }} to {{ $labels.target_runtime }} failed"

      - alert: HighMigrationRollbackRate
        expr: |
          (
            rate(aether_migration_rollbacks_total[5m])
            /
            rate(aether_migrations_total[5m])
          ) > 0.2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High migration rollback rate"
          description: "Migration rollback rate is {{ $value | humanizePercentage }}"

      - alert: RuntimeUnavailable
        expr: aether_runtime_available == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Runtime unavailable"
          description: "Runtime {{ $labels.runtime }} has been unavailable for 5 minutes"
```

## Best Practices

1. **Scrape Interval**: Set to 15-30 seconds for production workloads
2. **Retention**: Keep metrics for at least 15 days to track trends
3. **Labels**: Use runtime and status labels for filtering
4. **Alerts**: Configure alerts for:
   - High failure rates (>10%)
   - Migration failures
   - Runtime unavailability
   - Slow command execution (>30s)
5. **Dashboards**: Create separate dashboards for:
   - Overview (all workloads)
   - Per-runtime metrics
   - Migration performance
   - CLI usage patterns

## Troubleshooting

### Metrics Not Updating

Check that commands are being executed:
```bash
aether list
aether metrics | grep cli_commands_total
```

### Missing Labels

Ensure workloads are tracked in state:
```bash
cat ~/.aether/state.json
```

### Historical Data

Metrics are ephemeral (reset on restart). For persistence, use Prometheus scraping with proper retention.

## Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Dashboard Best Practices](https://grafana.com/docs/grafana/latest/dashboards/build-dashboards/best-practices/)
- [PromQL Examples](https://prometheus.io/docs/prometheus/latest/querying/examples/)
