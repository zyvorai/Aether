# Orchestr8 Helm Chart

This Helm chart deploys Orchestr8 as a Kubernetes operator for managing workloads across multiple runtimes (Podman, Kubernetes, KubeVirt, Metal3).

## Prerequisites

- Kubernetes 1.20+
- Helm 3.0+
- (Optional) KubeVirt CRDs for VM support
- (Optional) Metal3 CRDs for bare metal support

## Installation

### Add Helm Repository

```bash
# Add the Orchestr8 Helm repository (when published)
helm repo add orchestr8 https://ssahani.github.io/orchestr8/charts
helm repo update
```

### Install from Local Chart

```bash
# From the repository root
helm install orchestr8 ./helm/orchestr8

# Or with custom values
helm install orchestr8 ./helm/orchestr8 -f custom-values.yaml
```

### Install with Custom Namespace

```bash
helm install orchestr8 ./helm/orchestr8 \
  --namespace orchestr8-system \
  --create-namespace
```

## Configuration

### Basic Configuration

```yaml
# values.yaml
replicaCount: 1

image:
  repository: ghcr.io/ssahani/orchestr8
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

### Enable Ingress

```yaml
ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
  hosts:
    - host: orchestr8.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: orchestr8-tls
      hosts:
        - orchestr8.example.com
```

### Runtime Configuration

```yaml
orchestr8:
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

    metal3:
      enabled: false  # Requires Metal3 CRDs

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
- BareMetalHosts (Metal3)

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
apiVersion: orchestr8/v1
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
kubectl exec -it deployment/orchestr8 -- \
  orchestr8 -s /tmp/workload.yaml run
```

### Monitor with Prometheus

If `metrics.serviceMonitor.enabled` is true, Prometheus Operator will automatically scrape metrics.

Manual Prometheus configuration:

```yaml
scrape_configs:
  - job_name: 'orchestr8'
    kubernetes_sd_configs:
      - role: service
        namespaces:
          names:
            - orchestr8-system
    relabel_configs:
      - source_labels: [__meta_kubernetes_service_name]
        action: keep
        regex: orchestr8
```

### View Logs

```bash
kubectl logs -f deployment/orchestr8
```

### Access Metrics

```bash
# Port forward metrics endpoint
kubectl port-forward service/orchestr8 9090:9090

# Fetch metrics
curl http://localhost:9090/metrics
```

## Examples

### Example 1: Basic Installation

```bash
helm install orchestr8 ./helm/orchestr8 \
  --set replicaCount=1 \
  --set persistence.enabled=true \
  --set persistence.size=5Gi
```

### Example 2: With Metrics and Monitoring

```bash
helm install orchestr8 ./helm/orchestr8 \
  --set metrics.enabled=true \
  --set metrics.serviceMonitor.enabled=true \
  --set metrics.serviceMonitor.interval=15s
```

### Example 3: Production Configuration

```bash
helm install orchestr8 ./helm/orchestr8 -f - <<EOF
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
    - host: orchestr8.prod.example.com
      paths:
        - path: /
          pathType: Prefix

orchestr8:
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
                  - orchestr8
          topologyKey: kubernetes.io/hostname
EOF
```

### Example 4: Enable All Runtimes

```bash
helm install orchestr8 ./helm/orchestr8 -f - <<EOF
orchestr8:
  runtimes:
    podman:
      enabled: true
      socket: unix:///var/run/podman/podman.sock

    kubernetes:
      enabled: true
      inCluster: true

    kubevirt:
      enabled: true

    metal3:
      enabled: true
      bmcCredentials:
        secretName: metal3-bmc-credentials

secrets:
  bmcCredentials:
    username: admin
    password: changeme
EOF
```

## Upgrading

### Upgrade to New Version

```bash
helm upgrade orchestr8 ./helm/orchestr8
```

### Upgrade with New Values

```bash
helm upgrade orchestr8 ./helm/orchestr8 -f new-values.yaml
```

### Rollback

```bash
helm rollback orchestr8
```

## Uninstallation

```bash
helm uninstall orchestr8
```

To also delete PVCs:

```bash
kubectl delete pvc -l app.kubernetes.io/instance=orchestr8
```

## Values Reference

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of Orchestr8 replicas | `1` |
| `image.repository` | Container image repository | `ghcr.io/ssahani/orchestr8` |
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
| `orchestr8.namespace` | Default workload namespace | `default` |
| `orchestr8.logging.level` | Log level | `info` |
| `orchestr8.runtimes.kubernetes.enabled` | Enable Kubernetes runtime | `true` |
| `orchestr8.runtimes.kubevirt.enabled` | Enable KubeVirt runtime | `false` |
| `orchestr8.runtimes.metal3.enabled` | Enable Metal3 runtime | `false` |

For a complete list, see [`values.yaml`](values.yaml).

## Troubleshooting

### Pods Not Starting

Check pod status:
```bash
kubectl describe pod -l app.kubernetes.io/name=orchestr8
```

Check logs:
```bash
kubectl logs -l app.kubernetes.io/name=orchestr8
```

### Permission Errors

Verify RBAC is enabled:
```bash
kubectl get clusterrole orchestr8
kubectl get clusterrolebinding orchestr8
```

### Metrics Not Available

Check metrics endpoint:
```bash
kubectl port-forward service/orchestr8 9090:9090
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
helm lint ./helm/orchestr8

# Dry run
helm install orchestr8 ./helm/orchestr8 --dry-run --debug

# Template and review
helm template orchestr8 ./helm/orchestr8
```

### Package Chart

```bash
helm package ./helm/orchestr8
```

## Contributing

Contributions welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Proprietary (HyperSDK)

## Support

- GitHub Issues: https://github.com/ssahani/orchestr8/issues
- Documentation: https://github.com/ssahani/orchestr8
