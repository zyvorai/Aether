# Observability Stack Setup for Aether

Complete guide to deploying and configuring a production-grade observability stack for Aether workloads.

## Overview

This guide covers setting up a complete observability stack including:
- **Prometheus** - Metrics collection and storage
- **Grafana** - Visualization and dashboards
- **AlertManager** - Alert routing and notification
- **Loki** - Log aggregation
- **Jaeger** - Distributed tracing

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Observability Stack                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐    ┌──────────┐    ┌──────────────┐          │
│  │ Aether│───>│Prometheus│───>│ AlertManager │          │
│  │  Metrics │    │          │    │              │          │
│  └──────────┘    └────┬─────┘    └──────┬───────┘          │
│                       │                   │                  │
│                       │                   │                  │
│  ┌──────────┐        │                   │                  │
│  │ Workload │        │                   │                  │
│  │  Logs    │───>┌───▼─────┐    ┌────────▼────────┐        │
│  └──────────┘    │  Loki   │    │   Grafana       │        │
│                  │         │◄───┤   Dashboards    │        │
│  ┌──────────┐    └─────────┘    │   & Alerts      │        │
│  │ Traces   │                    └─────────────────┘        │
│  │          │───>┌─────────┐                                │
│  └──────────┘    │ Jaeger  │                                │
│                  └─────────┘                                 │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Deploy Observability Stack

```bash
# Create namespace
kubectl create namespace observability

# Deploy Prometheus
kubectl apply -f examples/observability/prometheus.yaml -n observability

# Deploy Grafana
kubectl apply -f examples/observability/grafana.yaml -n observability

# Deploy AlertManager
kubectl apply -f examples/observability/alertmanager.yaml -n observability

# Deploy Loki
kubectl apply -f examples/observability/loki.yaml -n observability

# Wait for pods to be ready
kubectl wait --for=condition=ready pod -l app=prometheus -n observability --timeout=300s
kubectl wait --for=condition=ready pod -l app=grafana -n observability --timeout=300s
```

### 2. Access Dashboards

```bash
# Port-forward Grafana
kubectl port-forward -n observability svc/grafana 3000:3000

# Access at http://localhost:3000
# Default credentials: admin / admin
```

### 3. Import Aether Dashboard

1. Open Grafana at http://localhost:3000
2. Login with default credentials (admin/admin)
3. Navigate to Dashboards → Import
4. Upload `grafana/dashboard.json`
5. Select Prometheus data source
6. Click Import

## Component Details

### Prometheus

**Purpose:** Metrics collection, storage, and querying

**Configuration:**

```yaml
# examples/observability/prometheus.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
      evaluation_interval: 15s
      external_labels:
        cluster: 'production'
        environment: 'prod'

    # AlertManager configuration
    alerting:
      alertmanagers:
      - static_configs:
        - targets:
          - alertmanager:9093

    # Load rules
    rule_files:
      - /etc/prometheus/rules/*.yml

    scrape_configs:
      # Aether metrics
      - job_name: 'aether'
        kubernetes_sd_configs:
        - role: pod
          namespaces:
            names:
            - aether-production
        relabel_configs:
        - source_labels: [__meta_kubernetes_pod_label_app]
          action: keep
          regex: aether
        - source_labels: [__meta_kubernetes_pod_name]
          target_label: pod
        - source_labels: [__meta_kubernetes_namespace]
          target_label: namespace

      # Kubernetes node metrics
      - job_name: 'kubernetes-nodes'
        kubernetes_sd_configs:
        - role: node
        scheme: https
        tls_config:
          ca_file: /var/run/secrets/kubernetes.io/serviceaccount/ca.crt
        bearer_token_file: /var/run/secrets/kubernetes.io/serviceaccount/token

      # Kubernetes pods
      - job_name: 'kubernetes-pods'
        kubernetes_sd_configs:
        - role: pod
        relabel_configs:
        - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
          action: keep
          regex: true
        - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
          action: replace
          target_label: __metrics_path__
          regex: (.+)
        - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
          action: replace
          regex: ([^:]+)(?::\d+)?;(\d+)
          replacement: $1:$2
          target_label: __address__

      # Kubernetes services
      - job_name: 'kubernetes-services'
        kubernetes_sd_configs:
        - role: service
        metrics_path: /probe
        params:
          module: [http_2xx]
        relabel_configs:
        - source_labels: [__meta_kubernetes_service_annotation_prometheus_io_probe]
          action: keep
          regex: true
```

**Storage Configuration:**

```yaml
# Persistent storage for Prometheus
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: prometheus-storage
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 100Gi
  storageClassName: standard
```

**Retention Policy:**

```yaml
# In Prometheus deployment
args:
  - '--config.file=/etc/prometheus/prometheus.yml'
  - '--storage.tsdb.path=/prometheus'
  - '--storage.tsdb.retention.time=30d'
  - '--storage.tsdb.retention.size=90GB'
  - '--web.enable-lifecycle'
  - '--web.enable-admin-api'
```

### Grafana

**Purpose:** Visualization, dashboards, and alerting UI

**Data Sources Configuration:**

```yaml
# examples/observability/grafana-datasources.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: grafana-datasources
data:
  datasources.yaml: |
    apiVersion: 1
    datasources:
    # Prometheus
    - name: Prometheus
      type: prometheus
      access: proxy
      url: http://prometheus:9090
      isDefault: true
      editable: true
      jsonData:
        timeInterval: 15s

    # Loki
    - name: Loki
      type: loki
      access: proxy
      url: http://loki:3100
      editable: true

    # Jaeger
    - name: Jaeger
      type: jaeger
      access: proxy
      url: http://jaeger-query:16686
      editable: true
```

**Dashboard Provisioning:**

```yaml
# examples/observability/grafana-dashboards.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: grafana-dashboard-provider
data:
  dashboards.yaml: |
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
        path: /var/lib/grafana/dashboards
```

**Notification Channels:**

```yaml
# Slack notification
apiVersion: v1
kind: ConfigMap
metadata:
  name: grafana-notifiers
data:
  notifiers.yaml: |
    notifiers:
    - name: Slack
      type: slack
      uid: slack-alerts
      org_id: 1
      is_default: true
      settings:
        url: https://hooks.slack.com/services/YOUR/WEBHOOK/URL
        recipient: '#alerts'
        username: Grafana

    - name: PagerDuty
      type: pagerduty
      uid: pagerduty
      org_id: 1
      settings:
        integrationKey: YOUR_INTEGRATION_KEY
        autoResolve: true
```

### AlertManager

**Purpose:** Alert routing, grouping, silencing, and notification

**Configuration:**

```yaml
# examples/observability/alertmanager-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: alertmanager-config
data:
  alertmanager.yml: |
    global:
      resolve_timeout: 5m
      slack_api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'

    # Templates for notifications
    templates:
    - '/etc/alertmanager/templates/*.tmpl'

    # Routing tree
    route:
      group_by: ['alertname', 'cluster', 'service']
      group_wait: 10s
      group_interval: 10s
      repeat_interval: 12h
      receiver: 'default'

      routes:
      # Critical alerts - immediate notification
      - match:
          severity: critical
        receiver: 'pagerduty-critical'
        continue: true

      - match:
          severity: critical
        receiver: 'slack-critical'

      # Warning alerts - Slack only
      - match:
          severity: warning
        receiver: 'slack-warnings'

      # Database alerts
      - match:
          service: database
        receiver: 'database-team'

      # Workload alerts
      - match_re:
          alertname: ^(WorkloadFailed|WorkloadCrashLoop)$
        receiver: 'aether-team'

    # Inhibition rules (suppress alerts)
    inhibit_rules:
    # Suppress warning if critical is firing
    - source_match:
        severity: 'critical'
      target_match:
        severity: 'warning'
      equal: ['alertname', 'cluster', 'service']

    # Receivers
    receivers:
    - name: 'default'
      slack_configs:
      - channel: '#general-alerts'
        title: 'Alert: {{ .GroupLabels.alertname }}'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

    - name: 'pagerduty-critical'
      pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_KEY'
        description: '{{ .GroupLabels.alertname }}'

    - name: 'slack-critical'
      slack_configs:
      - channel: '#critical-alerts'
        color: 'danger'
        title: ':fire: Critical Alert'
        text: |
          *Alert:* {{ .GroupLabels.alertname }}
          *Severity:* {{ .CommonLabels.severity }}
          *Description:* {{ .CommonAnnotations.description }}
          *Runbook:* {{ .CommonAnnotations.runbook_url }}

    - name: 'slack-warnings'
      slack_configs:
      - channel: '#warnings'
        color: 'warning'
        title: 'Warning Alert'

    - name: 'database-team'
      slack_configs:
      - channel: '#database-team'

    - name: 'aether-team'
      slack_configs:
      - channel: '#aether-alerts'
        title: 'Aether Alert'
```

**Alert Templates:**

```yaml
# examples/observability/alert-templates.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: alertmanager-templates
data:
  default.tmpl: |
    {{ define "slack.default.title" }}
    [{{ .Status | toUpper }}{{ if eq .Status "firing" }}:{{ .Alerts.Firing | len }}{{ end }}] {{ .GroupLabels.alertname }}
    {{ end }}

    {{ define "slack.default.text" }}
    {{ range .Alerts }}
    *Alert:* {{ .Labels.alertname }}
    *Severity:* {{ .Labels.severity }}
    *Summary:* {{ .Annotations.summary }}
    *Description:* {{ .Annotations.description }}
    {{ if .Labels.namespace }}*Namespace:* {{ .Labels.namespace }}{{ end }}
    {{ if .Labels.pod }}*Pod:* {{ .Labels.pod }}{{ end }}
    {{ if .Annotations.runbook_url }}*Runbook:* {{ .Annotations.runbook_url }}{{ end }}
    {{ end }}
    {{ end }}
```

### Alert Rules

**Aether-Specific Alerts:**

```yaml
# examples/observability/aether-alerts.yaml
groups:
- name: aether
  interval: 30s
  rules:
  # Workload failures
  - alert: WorkloadFailed
    expr: |
      aether_workload_operations_total{operation="run", status="failed"} > 0
    for: 5m
    labels:
      severity: critical
      component: aether
    annotations:
      summary: "Workload deployment failed"
      description: "Workload {{ $labels.workload }} failed to deploy"
      runbook_url: "https://github.com/zyvorai/Aether/blob/main/docs/RUNBOOK.md#workload-deployment-failure"

  - alert: WorkloadCrashLoop
    expr: |
      rate(aether_workload_operations_total{operation="stop", status="failed"}[5m]) > 0.1
    for: 10m
    labels:
      severity: critical
      component: aether
    annotations:
      summary: "Workload in crash loop"
      description: "Workload {{ $labels.workload }} is crash looping"

  # Migration issues
  - alert: MigrationFailed
    expr: |
      aether_migration_total{status="failed"} > 0
    for: 1m
    labels:
      severity: critical
      component: aether
    annotations:
      summary: "Workload migration failed"
      description: "Migration from {{ $labels.source_runtime }} to {{ $labels.target_runtime }} failed"

  - alert: HighMigrationRollbackRate
    expr: |
      rate(aether_migration_rollbacks_total[30m]) > 0.2
    for: 10m
    labels:
      severity: warning
      component: aether
    annotations:
      summary: "High migration rollback rate"
      description: "More than 20% of migrations are being rolled back"

  # Runtime availability
  - alert: RuntimeUnavailable
    expr: |
      aether_runtime_available == 0
    for: 5m
    labels:
      severity: critical
      component: aether
    annotations:
      summary: "Runtime unavailable"
      description: "Runtime {{ $labels.runtime }} is unavailable"

  # Performance degradation
  - alert: HighMigrationDuration
    expr: |
      histogram_quantile(0.95,
        rate(aether_migration_duration_seconds_bucket[5m])
      ) > 300
    for: 10m
    labels:
      severity: warning
      component: aether
    annotations:
      summary: "Migrations taking longer than expected"
      description: "p95 migration duration is {{ $value }}s (threshold: 300s)"

  # CLI command failures
  - alert: HighCLIFailureRate
    expr: |
      rate(aether_cli_commands_total{status="failed"}[5m])
      / rate(aether_cli_commands_total[5m]) > 0.1
    for: 15m
    labels:
      severity: warning
      component: aether
    annotations:
      summary: "High CLI command failure rate"
      description: "More than 10% of CLI commands are failing"
```

**Infrastructure Alerts:**

```yaml
# examples/observability/infrastructure-alerts.yaml
groups:
- name: kubernetes
  interval: 30s
  rules:
  # Node issues
  - alert: NodeNotReady
    expr: kube_node_status_condition{condition="Ready",status="true"} == 0
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Node not ready"
      description: "Node {{ $labels.node }} has been unready for 5 minutes"

  - alert: NodeHighCPU
    expr: |
      100 - (avg by (node) (irate(node_cpu_seconds_total{mode="idle"}[5m])) * 100) > 80
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "Node CPU usage high"
      description: "Node {{ $labels.node }} CPU usage is {{ $value }}%"

  - alert: NodeHighMemory
    expr: |
      (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100 > 85
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "Node memory usage high"
      description: "Node {{ $labels.node }} memory usage is {{ $value }}%"

  # Pod issues
  - alert: PodCrashLooping
    expr: |
      rate(kube_pod_container_status_restarts_total[15m]) > 0
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Pod crash looping"
      description: "Pod {{ $labels.namespace }}/{{ $labels.pod }} is crash looping"

  - alert: PodNotReady
    expr: |
      sum by (namespace, pod) (kube_pod_status_phase{phase!~"Running|Succeeded"}) > 0
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "Pod not ready"
      description: "Pod {{ $labels.namespace }}/{{ $labels.pod }} has been not ready for 10 minutes"

  # Persistent volumes
  - alert: PVCPendingBound
    expr: |
      kube_persistentvolumeclaim_status_phase{phase="Pending"} == 1
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "PVC pending binding"
      description: "PVC {{ $labels.namespace }}/{{ $labels.persistentvolumeclaim }} is pending for 10 minutes"

  - alert: PVCLowSpace
    expr: |
      (kubelet_volume_stats_available_bytes / kubelet_volume_stats_capacity_bytes) * 100 < 10
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "PVC running out of space"
      description: "PVC {{ $labels.namespace }}/{{ $labels.persistentvolumeclaim }} has less than 10% space remaining"
```

**Application Alerts:**

```yaml
# examples/observability/application-alerts.yaml
groups:
- name: application
  interval: 30s
  rules:
  # High error rate
  - alert: HighErrorRate
    expr: |
      sum(rate(http_requests_total{status=~"5.."}[5m])) by (service)
      / sum(rate(http_requests_total[5m])) by (service) > 0.05
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate detected"
      description: "Service {{ $labels.service }} error rate is {{ $value | humanizePercentage }}"

  # High latency
  - alert: HighLatency
    expr: |
      histogram_quantile(0.95,
        sum(rate(http_request_duration_seconds_bucket[5m])) by (le, service)
      ) > 1
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "High latency detected"
      description: "Service {{ $labels.service }} p95 latency is {{ $value }}s"

  # Low throughput
  - alert: LowThroughput
    expr: |
      sum(rate(http_requests_total[5m])) by (service) < 10
    for: 15m
    labels:
      severity: warning
    annotations:
      summary: "Low request throughput"
      description: "Service {{ $labels.service }} is receiving < 10 req/s"

  # Database connection pool exhaustion
  - alert: DatabaseConnectionPoolExhausted
    expr: |
      (db_connections_in_use / db_connections_max) > 0.9
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Database connection pool exhausted"
      description: "Database {{ $labels.database }} connection pool is {{ $value | humanizePercentage }} full"

  # Cache hit rate low
  - alert: LowCacheHitRate
    expr: |
      rate(cache_hits_total[5m])
      / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m])) < 0.8
    for: 15m
    labels:
      severity: warning
    annotations:
      summary: "Low cache hit rate"
      description: "Cache hit rate is {{ $value | humanizePercentage }}"
```

### Loki (Log Aggregation)

**Configuration:**

```yaml
# examples/observability/loki-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: loki-config
data:
  loki.yaml: |
    auth_enabled: false

    server:
      http_listen_port: 3100

    ingester:
      lifecycler:
        address: 127.0.0.1
        ring:
          kvstore:
            store: inmemory
          replication_factor: 1
      chunk_idle_period: 5m
      chunk_retain_period: 30s

    schema_config:
      configs:
      - from: 2024-01-01
        store: boltdb-shipper
        object_store: filesystem
        schema: v11
        index:
          prefix: index_
          period: 24h

    storage_config:
      boltdb_shipper:
        active_index_directory: /loki/index
        cache_location: /loki/cache
        shared_store: filesystem
      filesystem:
        directory: /loki/chunks

    limits_config:
      enforce_metric_name: false
      reject_old_samples: true
      reject_old_samples_max_age: 168h
      ingestion_rate_mb: 10
      ingestion_burst_size_mb: 20

    chunk_store_config:
      max_look_back_period: 0s

    table_manager:
      retention_deletes_enabled: true
      retention_period: 720h  # 30 days
```

**Promtail (Log Shipper):**

```yaml
# examples/observability/promtail-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: promtail-config
data:
  promtail.yaml: |
    server:
      http_listen_port: 9080
      grpc_listen_port: 0

    positions:
      filename: /tmp/positions.yaml

    clients:
      - url: http://loki:3100/loki/api/v1/push

    scrape_configs:
    # Kubernetes pod logs
    - job_name: kubernetes-pods
      kubernetes_sd_configs:
      - role: pod

      relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        target_label: app
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: pod
      - source_labels: [__meta_kubernetes_namespace]
        target_label: namespace
      - source_labels: [__meta_kubernetes_pod_container_name]
        target_label: container

      pipeline_stages:
      # Parse JSON logs
      - json:
          expressions:
            level: level
            message: msg
            timestamp: time

      # Extract log level
      - labels:
          level:

      # Parse timestamp
      - timestamp:
          source: timestamp
          format: RFC3339
```

### Jaeger (Distributed Tracing)

**Configuration:**

```yaml
# examples/observability/jaeger.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: jaeger
spec:
  replicas: 1
  selector:
    matchLabels:
      app: jaeger
  template:
    metadata:
      labels:
        app: jaeger
    spec:
      containers:
      - name: jaeger
        image: jaegertracing/all-in-one:1.51
        env:
        - name: COLLECTOR_ZIPKIN_HOST_PORT
          value: ":9411"
        - name: SPAN_STORAGE_TYPE
          value: "badger"
        - name: BADGER_EPHEMERAL
          value: "false"
        - name: BADGER_DIRECTORY_VALUE
          value: "/badger/data"
        - name: BADGER_DIRECTORY_KEY
          value: "/badger/key"
        ports:
        - containerPort: 5775
          protocol: UDP
        - containerPort: 6831
          protocol: UDP
        - containerPort: 6832
          protocol: UDP
        - containerPort: 5778
          protocol: TCP
        - containerPort: 16686
          protocol: TCP
        - containerPort: 14268
          protocol: TCP
        - containerPort: 14250
          protocol: TCP
        - containerPort: 9411
          protocol: TCP
        volumeMounts:
        - name: data
          mountPath: /badger
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: jaeger-storage
```

## Complete Deployment

### Deploy All Components

```bash
#!/bin/bash
# deploy-observability.sh

set -e

NAMESPACE="observability"

echo "Deploying observability stack to ${NAMESPACE}..."

# Create namespace
kubectl create namespace ${NAMESPACE} --dry-run=client -o yaml | kubectl apply -f -

# Deploy Prometheus
echo "Deploying Prometheus..."
kubectl apply -f prometheus-config.yaml -n ${NAMESPACE}
kubectl apply -f prometheus-rules.yaml -n ${NAMESPACE}
kubectl apply -f prometheus.yaml -n ${NAMESPACE}

# Deploy AlertManager
echo "Deploying AlertManager..."
kubectl apply -f alertmanager-config.yaml -n ${NAMESPACE}
kubectl apply -f alertmanager-templates.yaml -n ${NAMESPACE}
kubectl apply -f alertmanager.yaml -n ${NAMESPACE}

# Deploy Grafana
echo "Deploying Grafana..."
kubectl apply -f grafana-datasources.yaml -n ${NAMESPACE}
kubectl apply -f grafana-dashboards.yaml -n ${NAMESPACE}
kubectl apply -f grafana.yaml -n ${NAMESPACE}

# Deploy Loki and Promtail
echo "Deploying Loki..."
kubectl apply -f loki-config.yaml -n ${NAMESPACE}
kubectl apply -f loki.yaml -n ${NAMESPACE}
kubectl apply -f promtail-config.yaml -n ${NAMESPACE}
kubectl apply -f promtail.yaml -n ${NAMESPACE}

# Deploy Jaeger
echo "Deploying Jaeger..."
kubectl apply -f jaeger.yaml -n ${NAMESPACE}

echo "Waiting for pods to be ready..."
kubectl wait --for=condition=ready pod -l app=prometheus -n ${NAMESPACE} --timeout=300s
kubectl wait --for=condition=ready pod -l app=grafana -n ${NAMESPACE} --timeout=300s
kubectl wait --for=condition=ready pod -l app=alertmanager -n ${NAMESPACE} --timeout=300s
kubectl wait --for=condition=ready pod -l app=loki -n ${NAMESPACE} --timeout=300s
kubectl wait --for=condition=ready pod -l app=jaeger -n ${NAMESPACE} --timeout=300s

echo "Observability stack deployed successfully!"
echo ""
echo "Access URLs (use kubectl port-forward):"
echo "  Grafana:      kubectl port-forward -n ${NAMESPACE} svc/grafana 3000:3000"
echo "  Prometheus:   kubectl port-forward -n ${NAMESPACE} svc/prometheus 9090:9090"
echo "  AlertManager: kubectl port-forward -n ${NAMESPACE} svc/alertmanager 9093:9093"
echo "  Jaeger:       kubectl port-forward -n ${NAMESPACE} svc/jaeger-query 16686:16686"
```

## Monitoring Best Practices

### 1. Metric Naming

Follow Prometheus naming conventions:

```
aether_<subsystem>_<metric_name>_<unit>

Examples:
- aether_workload_operations_total (counter)
- aether_migration_duration_seconds (histogram)
- aether_runtime_available (gauge)
```

### 2. Label Usage

Use labels for dimensions, not for high-cardinality data:

```promql
# Good - bounded labels
aether_workload_operations_total{operation="run", status="success", runtime="kubernetes"}

# Bad - unbounded labels
aether_workload_operations_total{workload_name="user-123-app"}
```

### 3. Alert Design

**Golden Rules:**
- Alerts should be actionable
- Include runbook links
- Use appropriate severity levels
- Set proper thresholds and durations

```yaml
- alert: Example
  expr: metric > threshold
  for: 10m  # Don't alert on transient issues
  labels:
    severity: critical  # critical, warning, info
  annotations:
    summary: "Short description"
    description: "Detailed description with context"
    runbook_url: "Link to troubleshooting steps"
```

### 4. Dashboard Design

**Principles:**
- Use RED method: Rate, Errors, Duration
- USE method for resources: Utilization, Saturation, Errors
- Organize by user journey
- Include SLO/SLI indicators

### 5. Log Levels

Use appropriate log levels:

```
ERROR - Actionable errors requiring attention
WARN  - Potential issues, degraded performance
INFO  - Important business events
DEBUG - Detailed diagnostic information
TRACE - Very detailed diagnostic information
```

### 6. Retention Policies

Set appropriate retention based on storage and compliance:

```yaml
Metrics (Prometheus):
  - Short-term: 30 days (raw data)
  - Long-term: 1 year (downsampled)

Logs (Loki):
  - Application logs: 30 days
  - Audit logs: 90 days
  - Debug logs: 7 days

Traces (Jaeger):
  - Recent traces: 7 days
  - Sampled traces: 30 days
```

## Troubleshooting

### Prometheus Not Scraping

```bash
# Check service discovery
kubectl port-forward -n observability svc/prometheus 9090:9090
# Visit http://localhost:9090/targets

# Check pod annotations
kubectl get pod <pod-name> -o yaml | grep -A 5 annotations

# Verify metrics endpoint
kubectl exec -n <namespace> <pod-name> -- curl localhost:5090/metrics
```

### Grafana Data Source Issues

```bash
# Test Prometheus connection from Grafana pod
kubectl exec -n observability <grafana-pod> -- curl http://prometheus:9090/api/v1/query?query=up

# Check Grafana logs
kubectl logs -n observability <grafana-pod> | grep -i error
```

### AlertManager Not Sending Alerts

```bash
# Check AlertManager configuration
kubectl port-forward -n observability svc/alertmanager 9093:9093
# Visit http://localhost:9093/#/status

# Verify webhook endpoints
kubectl exec -n observability <alertmanager-pod> -- \
  wget -O- --post-data='{}' http://localhost:9093/-/reload

# Check alerts in Prometheus
# Visit http://localhost:9090/alerts
```

### Loki Not Receiving Logs

```bash
# Check Promtail logs
kubectl logs -n observability <promtail-pod>

# Verify Loki is accessible
kubectl exec -n observability <promtail-pod> -- \
  curl http://loki:3100/ready

# Query Loki directly
kubectl exec -n observability <loki-pod> -- \
  wget -O- 'http://localhost:3100/loki/api/v1/query?query={app="myapp"}'
```

## Performance Tuning

### Prometheus

```yaml
# Increase resources for large deployments
resources:
  requests:
    memory: 4Gi
    cpu: 2
  limits:
    memory: 8Gi
    cpu: 4

# Tune TSDB
args:
  - --storage.tsdb.min-block-duration=2h
  - --storage.tsdb.max-block-duration=2h
  - --storage.tsdb.wal-compression
```

### Grafana

```yaml
# Enable caching
env:
- name: GF_RENDERING_SERVER_URL
  value: http://renderer:8081/render
- name: GF_RENDERING_CALLBACK_URL
  value: http://grafana:3000/

# Database backend (for HA)
- name: GF_DATABASE_TYPE
  value: postgres
- name: GF_DATABASE_HOST
  value: postgres:5432
```

### Loki

```yaml
# Increase ingestion limits
limits_config:
  ingestion_rate_mb: 50
  ingestion_burst_size_mb: 100
  max_streams_per_user: 10000
  max_query_length: 721h
```

## Security

### Authentication

```yaml
# Grafana OAuth
env:
- name: GF_AUTH_GENERIC_OAUTH_ENABLED
  value: "true"
- name: GF_AUTH_GENERIC_OAUTH_CLIENT_ID
  value: "grafana"
- name: GF_AUTH_GENERIC_OAUTH_CLIENT_SECRET
  valueFrom:
    secretKeyRef:
      name: oauth-secret
      key: client-secret
```

### TLS Configuration

```yaml
# Prometheus TLS
args:
  - --web.config.file=/etc/prometheus/web-config.yml

# web-config.yml
tls_server_config:
  cert_file: /etc/prometheus/certs/tls.crt
  key_file: /etc/prometheus/certs/tls.key
```

### RBAC

```yaml
# Prometheus ServiceAccount
apiVersion: v1
kind: ServiceAccount
metadata:
  name: prometheus
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: prometheus
rules:
- apiGroups: [""]
  resources:
  - nodes
  - nodes/proxy
  - services
  - endpoints
  - pods
  verbs: ["get", "list", "watch"]
```

## Support

For issues and questions:
- GitHub Issues: https://github.com/zyvorai/Aether/issues
- Tag: `observability` or `monitoring`
- Documentation: https://github.com/zyvorai/Aether/tree/main/docs
