# Aether Helm Chart

This Helm chart deploys Aether as a Kubernetes operator for managing workloads across multiple runtimes (Podman, Kubernetes, KubeVirt).

## Prerequisites

- Kubernetes 1.20+
- Helm 3.0+
- (Optional) KubeVirt CRDs for VM support

## Installation

```bash
# From OCI (when charts are published)
helm install aether oci://ghcr.io/zyvorai/charts/aether --version 0.4.0

# From a local checkout
helm install aether ./helm/aether

# Custom values / namespace
helm install aether ./helm/aether -f custom-values.yaml \
  --namespace aether-system \
  --create-namespace
```

## Configuration

### Basic Configuration

```yaml
# values.yaml
replicaCount: 1

image:
  repository: ghcr.io/zyvorai/aether
  tag: "0.1.0"
  pullPolicy: IfNotPresent

resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: 500m
    memory: 512Mi
```

### Enable Metrics

```yaml
metrics:
  enabled: true
  port: 9090
  serviceMonitor:
    enabled: true
    interval: 30s
```

When Prometheus Operator scrapes the ServiceMonitor, set on the Aether deployment:

```yaml
env:
  - name: AETHER_PROMETHEUS_URL
    value: "http://prometheus-server.monitoring.svc:9090"
  - name: AETHER_GRAFANA_URL
    value: "https://grafana.example.com"
  - name: AETHER_GRAFANA_DASHBOARD_UID
    value: "<uid-from-import>"
```

Import the bundled dashboard from the repo root:

```bash
GRAFANA_URL=https://grafana.example.com GRAFANA_API_KEY=... ./scripts/import-grafana-dashboard.sh
```

RBAC includes `cilium.io` policy rules when Cilium is your CNI — see `values.yaml` → `rbac.rules`.

See [docs/guides/kubernetes/CILIUM.md](../../docs/guides/kubernetes/CILIUM.md) for Cilium bootstrap and Platform visibility.

### Enable Ingress

```yaml
ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: aether.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: aether-tls
      hosts:
        - aether.example.com
```

### Runtime Configuration

```yaml
aether:
  namespace: default

  logging:
    level: info
    format: json

  runtimes:
    podman:
      enabled: true

    kubernetes:
      enabled: true
      inCluster: true

    kubevirt:
      enabled: true  # Requires KubeVirt CRDs

  migration:
    defaultStrategy: blue-green
    validationDelay: 30s
    rollbackOnFailure: true
```

### Persistence

```yaml
persistence:
  enabled: true
  storageClass: "fast-ssd"
  accessMode: ReadWriteOnce
  size: 10Gi
```

### RBAC Permissions

The chart creates a ClusterRole with permissions for:
- Pods, Services, PVCs, ConfigMaps, Secrets
- Deployments, StatefulSets, DaemonSets
- Ingresses
- HorizontalPodAutoscalers
- VirtualMachines (KubeVirt)

To customize permissions:

```yaml
rbac:
  create: true
  rules:
    - apiGroups: [""]
      resources: ["pods"]
      verbs: ["get", "list", "watch"]
    # Add custom rules...
```

## Usage

### Deploy a Workload

```bash
# Create workload specification
cat > workload.yaml <<EOF
apiVersion: aether/v1
kind: Workload
metadata:
  name: my-app
  owner: team
  project: production
build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/myorg
requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "20Gi"
runtime:
  preferred: auto
  allow:
    - kube
    - kubevirt
EOF

# Deploy using kubectl exec
kubectl exec -it deployment/aether -- \
  aether -s /tmp/workload.yaml run
```

### Monitor with Prometheus

If `metrics.serviceMonitor.enabled` is true, Prometheus Operator will automatically scrape metrics.

Manual Prometheus configuration:

```yaml
scrape_configs:
  - job_name: 'aether'
    kubernetes_sd_configs:
      - role: service
        namespaces:
          names:
            - aether-system
    relabel_configs:
      - source_labels: [__meta_kubernetes_service_name]
        action: keep
        regex: aether
```

### View Logs

```bash
kubectl logs -f deployment/aether
```

### Access Metrics

```bash
# Port forward metrics endpoint
kubectl port-forward service/aether 9090:9090

# Fetch metrics
curl http://localhost:9090/metrics
```

## Examples

### Example 1: Basic Installation

```bash
helm install aether ./helm/aether \
  --set replicaCount=1 \
  --set persistence.enabled=true \
  --set persistence.size=5Gi
```

### Example 2: With Metrics and Monitoring

```bash
helm install aether ./helm/aether \
  --set metrics.enabled=true \
  --set metrics.serviceMonitor.enabled=true \
  --set metrics.serviceMonitor.interval=15s
```

### Example 3: Production Configuration

```bash
helm install aether ./helm/aether -f - <<EOF
replicaCount: 3

resources:
  requests:
    cpu: 200m
    memory: 256Mi
  limits:
    cpu: 1000m
    memory: 1Gi

autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70

persistence:
  enabled: true
  storageClass: fast-ssd
  size: 20Gi

metrics:
  enabled: true
  serviceMonitor:
    enabled: true

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: aether.prod.example.com
      paths:
        - path: /
          pathType: Prefix

aether:
  logging:
    level: info
    format: json

  runtimes:
    kubernetes:
      enabled: true
    kubevirt:
      enabled: true

  migration:
    defaultStrategy: blue-green
    validationDelay: 60s
    rollbackOnFailure: true

affinity:
  podAntiAffinity:
    preferredDuringSchedulingIgnoredDuringExecution:
      - weight: 100
        podAffinityTerm:
          labelSelector:
            matchExpressions:
              - key: app.kubernetes.io/name
                operator: In
                values:
                  - aether
          topologyKey: kubernetes.io/hostname
EOF
```

### Example 4: Enable All Runtimes

```bash
helm install aether ./helm/aether -f - <<EOF
aether:
  runtimes:
    podman:
      enabled: true
      socket: unix:///var/run/podman/podman.sock

    kubernetes:
      enabled: true
      inCluster: true

    kubevirt:
      enabled: true
EOF
```

## Upgrading

### Upgrade to New Version

```bash
helm upgrade aether ./helm/aether
```

### Upgrade with New Values

```bash
helm upgrade aether ./helm/aether -f new-values.yaml
```

### Rollback

```bash
helm rollback aether
```

## Uninstallation

```bash
helm uninstall aether
```

To also delete PVCs:

```bash
kubectl delete pvc -l app.kubernetes.io/instance=aether
```

## Values Reference

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of Aether replicas | `1` |
| `image.repository` | Container image repository | `ghcr.io/zyvorai/aether` |
| `image.tag` | Container image tag | `Chart.appVersion` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `serviceAccount.create` | Create service account | `true` |
| `serviceAccount.name` | Service account name | Auto-generated |
| `rbac.create` | Create RBAC resources | `true` |
| `service.type` | Service type | `ClusterIP` |
| `service.port` | Service port | `8080` |
| `metrics.enabled` | Enable Prometheus metrics | `true` |
| `metrics.port` | Metrics port | `9090` |
| `metrics.serviceMonitor.enabled` | Create ServiceMonitor | `false` |
| `ingress.enabled` | Enable ingress | `false` |
| `ingress.className` | Ingress class name | `""` |
| `resources.requests.cpu` | CPU request | `100m` |
| `resources.requests.memory` | Memory request | `128Mi` |
| `resources.limits.cpu` | CPU limit | `500m` |
| `resources.limits.memory` | Memory limit | `512Mi` |
| `autoscaling.enabled` | Enable HPA | `false` |
| `autoscaling.minReplicas` | Minimum replicas | `1` |
| `autoscaling.maxReplicas` | Maximum replicas | `10` |
| `persistence.enabled` | Enable persistence | `true` |
| `persistence.size` | PVC size | `5Gi` |
| `persistence.storageClass` | Storage class | `""` |
| `aether.namespace` | Default workload namespace | `default` |
| `aether.logging.level` | Log level | `info` |
| `aether.runtimes.kubernetes.enabled` | Enable Kubernetes runtime | `true` |
| `aether.runtimes.kubevirt.enabled` | Enable KubeVirt runtime | `false` |

For a complete list, see [`values.yaml`](values.yaml).

## Troubleshooting

### Pods Not Starting

Check pod status:
```bash
kubectl describe pod -l app.kubernetes.io/name=aether
```

Check logs:
```bash
kubectl logs -l app.kubernetes.io/name=aether
```

### Permission Errors

Verify RBAC is enabled:
```bash
kubectl get clusterrole aether
kubectl get clusterrolebinding aether
```

### Metrics Not Available

Check metrics endpoint:
```bash
kubectl port-forward service/aether 9090:9090
curl http://localhost:9090/metrics
```

Verify ServiceMonitor (if using Prometheus Operator):
```bash
kubectl get servicemonitor
```

## Development

### Test Chart Locally

```bash
# Lint chart
helm lint ./helm/aether

# Dry run
helm install aether ./helm/aether --dry-run --debug

# Template and review
helm template aether ./helm/aether
```

### Package Chart

```bash
helm package ./helm/aether
```

## Contributing

Contributions welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Proprietary (HyperSDK)

## Support

- GitHub Issues: https://github.com/zyvorai/Aether/issues
- Documentation: https://github.com/zyvorai/Aether
