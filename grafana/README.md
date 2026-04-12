# Aether Grafana Dashboard

This directory contains a pre-built Grafana dashboard for monitoring Aether workloads, migrations, and system health.

## Features

The dashboard includes:

### Overview Stats (Top Row)
- **Total Workloads**: Current number of running workloads
- **Deployment Success Rate**: Success rate over the last hour
- **Total Migrations**: Cumulative migration count
- **Migration Success Rate**: Overall migration reliability
- **Avg Migration Duration**: Performance metric for migrations
- **Version**: Current Aether version

### Visualization Panels

1. **Workloads by Runtime** (Pie Chart)
   - Distribution of workloads across Podman, Kubernetes, KubeVirt, Metal3
   - Percentage and absolute values

2. **Deployment Rate by Runtime** (Time Series)
   - Success and failure trends over time
   - Per-runtime breakdown
   - Statistics: mean, last, max

3. **Migration Operations** (Stacked Time Series)
   - Migration success/failure rates
   - Grouped by source → target runtime and strategy
   - Color-coded: green (success), red (failure)

4. **Migration Duration** (Time Series)
   - 95th and 50th percentile latencies
   - Helps identify performance degradation
   - Per-strategy breakdown

5. **Command Execution Latency** (Bar Chart)
   - 95th percentile CLI command duration
   - Identifies slow operations

6. **CLI Command Usage** (Time Series)
   - Frequency of each command
   - Usage patterns over time

7. **Runtime Availability** (Bar Gauge)
   - Real-time status of each runtime
   - Green (available) / Red (unavailable)

8. **Failures & Rollbacks** (Time Series)
   - Build failures
   - Deployment failures
   - Migration failures
   - Rollback events
   - All in the last hour

## Installation

### Prerequisites

- Grafana 8.0+ installed
- Prometheus datasource configured in Grafana
- Aether metrics being scraped by Prometheus

### Import Dashboard

#### Method 1: Grafana UI

1. Open Grafana web interface
2. Navigate to **Dashboards** → **Import**
3. Click **Upload JSON file**
4. Select `dashboard.json` from this directory
5. Select your Prometheus datasource
6. Click **Import**

#### Method 2: Grafana API

```bash
# Set your Grafana URL and API key
GRAFANA_URL="http://localhost:3000"
GRAFANA_API_KEY="your-api-key"

# Import dashboard
curl -X POST \
  -H "Authorization: Bearer ${GRAFANA_API_KEY}" \
  -H "Content-Type: application/json" \
  -d @grafana/dashboard.json \
  "${GRAFANA_URL}/api/dashboards/db"
```

#### Method 3: Provisioning

For automated deployments, add to Grafana provisioning:

```yaml
# /etc/grafana/provisioning/dashboards/aether.yaml
apiVersion: 1

providers:
  - name: 'Aether'
    orgId: 1
    folder: 'Aether'
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /var/lib/grafana/dashboards/aether
```

Copy dashboard:
```bash
sudo mkdir -p /var/lib/grafana/dashboards/aether
sudo cp grafana/dashboard.json /var/lib/grafana/dashboards/aether/
sudo chown -R grafana:grafana /var/lib/grafana/dashboards
```

## Configuration

### Datasource

The dashboard uses a template variable `${DS_PROMETHEUS}` for the Prometheus datasource.

During import, select your Prometheus datasource from the dropdown.

To change datasource later:
1. Open dashboard settings (gear icon)
2. Go to **Variables**
3. Edit `DS_PROMETHEUS`
4. Select different datasource

### Refresh Rate

Default: **30 seconds**

To change:
1. Click time picker in top right
2. Select refresh interval
3. Options: 5s, 10s, 30s, 1m, 5m, 15m, 30m, 1h

### Time Range

Default: **Last 1 hour**

Common ranges:
- Last 5 minutes (quick check)
- Last 15 minutes (detailed monitoring)
- Last 1 hour (overview)
- Last 6 hours (trend analysis)
- Last 24 hours (daily patterns)

## Prometheus Configuration

Ensure Prometheus is scraping Aether metrics:

### Node Exporter Textfile Collector

```bash
# Create metrics exporter script
cat > /usr/local/bin/aether-metrics.sh <<'EOF'
#!/bin/bash
METRICS_DIR="/var/lib/node_exporter/textfile_collector"
mkdir -p "${METRICS_DIR}"
aether metrics > "${METRICS_DIR}/aether.prom.$$"
mv "${METRICS_DIR}/aether.prom.$$" "${METRICS_DIR}/aether.prom"
EOF

chmod +x /usr/local/bin/aether-metrics.sh

# Add to cron (every 5 minutes)
echo "*/5 * * * * /usr/local/bin/aether-metrics.sh" | crontab -
```

### Prometheus prometheus.yml

```yaml
scrape_configs:
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']
    metric_relabel_configs:
      - source_labels: [__name__]
        regex: 'aether_.*'
        action: keep
```

Reload Prometheus:
```bash
curl -X POST http://localhost:9090/-/reload
# or
systemctl reload prometheus
```

## Alerts

The dashboard includes visual indicators for failures but doesn't create Prometheus alerts.

For alerting, see `docs/METRICS.md` for sample Prometheus alert rules.

Example alert integration with this dashboard:

1. Configure Prometheus alerts (see `docs/METRICS.md`)
2. Add alertmanager to Grafana:
   - **Configuration** → **Data Sources** → **Add data source** → **Alertmanager**
   - URL: `http://localhost:9093`
3. Alert annotations will appear on the dashboard timeline

## Customization

### Adding Panels

1. Click **Add panel** in edit mode
2. Use these common queries:

**Deployment success rate by runtime:**
```promql
rate(aether_workload_deployments_total{status="success"}[5m])
/
rate(aether_workload_deployments_total[5m])
```

**Migration rollback rate:**
```promql
rate(aether_migration_rollbacks_total[5m])
/
rate(aether_migrations_total[5m])
```

**Runtime decision latency:**
```promql
histogram_quantile(0.99, rate(aether_runtime_decision_seconds_bucket[5m]))
```

### Panel Colors

Recommended color scheme:
- **Success**: Green (#73BF69)
- **Failure**: Red (#F2495C)
- **Warning**: Yellow (#FADE2A)
- **Info**: Blue (#5794F2)

### Thresholds

Suggested thresholds for panels:

| Metric | Green | Yellow | Red |
|--------|-------|--------|-----|
| Deployment Success Rate | >95% | 90-95% | <90% |
| Migration Success Rate | >90% | 80-90% | <80% |
| Migration Duration | <30s | 30-60s | >60s |
| Command Latency | <5s | 5-10s | >10s |

## Troubleshooting

### Dashboard Shows "No Data"

1. **Check Prometheus scraping:**
   ```bash
   curl http://localhost:9090/api/v1/query?query=aether_build_info
   ```

2. **Verify metrics are being generated:**
   ```bash
   aether metrics | grep aether_
   ```

3. **Check Prometheus targets:**
   - Open http://localhost:9090/targets
   - Verify node-exporter is UP

4. **Check datasource in Grafana:**
   - Settings → Data Sources → Prometheus
   - Test connection

### Panels Show "N/A"

- No data in time range selected
- Metrics not scraped yet (wait for scrape interval)
- Query returning no results (check PromQL syntax)

### Slow Dashboard Performance

1. Increase scrape interval (15s → 30s)
2. Reduce time range (24h → 1h)
3. Increase refresh interval (5s → 30s)
4. Simplify queries (remove unnecessary labels)

### Missing Metrics

Ensure all operations are tracked:
```bash
# Run some operations
aether -s workload.yaml build
aether -s workload.yaml run
aether list

# Check metrics updated
aether metrics | grep -E "(builds|deployments|commands)_total"
```

## Screenshots

(Add screenshots after importing to Grafana)

### Overview
![Dashboard Overview](./screenshots/overview.png)

### Runtime Distribution
![Runtime Distribution](./screenshots/runtime-distribution.png)

### Migration Performance
![Migration Performance](./screenshots/migration-performance.png)

## Export & Sharing

### Export Dashboard

1. Dashboard settings → JSON Model
2. Copy JSON
3. Save to file

### Share Snapshot

1. Click **Share** icon (top right)
2. **Snapshot** tab
3. Set expiration
4. Click **Publish to snapshot.raintank.io**
5. Share URL

### Export as PDF

Requires Grafana Enterprise or Image Renderer plugin:

1. Install renderer:
   ```bash
   grafana-cli plugins install grafana-image-renderer
   systemctl restart grafana-server
   ```

2. Dashboard → **Share** → **Export** → **Save as PDF**

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-02-06 | Initial dashboard with 14 panels |

## Support

For issues or feature requests:
- GitHub: https://github.com/ssahani/aether/issues
- Documentation: `docs/METRICS.md`

## Related Resources

- [Prometheus Metrics Documentation](../docs/METRICS.md)
- [Aether README](../README.md)
- [Grafana Documentation](https://grafana.com/docs/)
- [PromQL Examples](https://prometheus.io/docs/prometheus/latest/querying/examples/)
