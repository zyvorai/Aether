# Kubernetes Runtime Guide

This guide explains how to use Aether with Kubernetes clusters.

## Prerequisites

1. **Access to a Kubernetes cluster**
   - Minikube (local)
   - Kind (local)
   - GKE, EKS, AKS (cloud)
   - Any k8s 1.20+ cluster

2. **kubectl configured**
   ```bash
   kubectl cluster-info
   kubectl get nodes
   ```

3. **Image registry access**
   - Docker Hub
   - GitHub Container Registry (ghcr.io)
   - Google Container Registry (gcr.io)
   - Amazon ECR, Azure ACR, etc.

## Quick Start

### 1. Configure Namespace (Optional)

By default, Aether uses the `default` namespace. You can override it with the `-n` / `--namespace` CLI flag or the `AETHER_NAMESPACE` environment variable:

```bash
# Option A: CLI flag (highest priority)
aether -n my-namespace run --runtime kube

# Option B: Environment variable
export AETHER_NAMESPACE=my-namespace

# Option C: Per-command environment variable
AETHER_NAMESPACE=staging aether run --runtime kube
```

Or create a custom namespace first:

```bash
kubectl create namespace aether-demo
aether -n aether-demo run --runtime kube
```

**Namespace resolution order:** `--namespace` flag > `AETHER_NAMESPACE` env var > `"default"`.

### 2. Prepare Your Image

**Option A: Use Podman to build and push**

```bash
# Build locally
podman build -t docker.io/yourorg/web-app:latest .

# Push to registry
podman push docker.io/yourorg/web-app:latest
```

**Option B: Use Docker**

```bash
docker build -t docker.io/yourorg/web-app:latest .
docker push docker.io/yourorg/web-app:latest
```

### 3. Create Workload Spec

See `workload-k8s.yaml` for a complete example:

```yaml
apiVersion: aether/v1
kind: Workload

metadata:
  name: web-app
  owner: yourname
  project: demo

build:
  registry: docker.io/yourorg/aether

runtime:
  preferred: kube
  allow:
    - kube

network:
  service: true
  serviceType: ClusterIP
  ports:
    - containerPort: 80
      servicePort: 8080
```

### 4. Deploy to Kubernetes

```bash
# Validate spec
aether validate --spec workload-k8s.yaml

# Deploy to cluster
aether run --spec workload-k8s.yaml

# Or explicitly select Kubernetes runtime
aether run --spec workload-k8s.yaml --runtime kube
```

### 5. Check Status

```bash
# Get workload status
aether status web-app

# Or use kubectl directly
kubectl get pods -l app=web-app
kubectl get svc web-app-service
```

### 6. View Logs

```bash
# Stream logs
aether logs web-app --follow

# Or use kubectl
kubectl logs -l app=web-app -f
```

### 7. Delete Resources

```bash
# Clean up all resources (Pod, Service, PVC)
aether delete web-app

# Verify deletion
kubectl get all -l managed-by=aether
```

## What Aether Creates

When you deploy a workload to Kubernetes, Aether automatically creates:

### 1. Pod

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: web-app
  labels:
    app: web-app
    managed-by: aether
spec:
  containers:
  - name: web-app
    image: docker.io/yourorg/web-app:latest
    ports:
    - containerPort: 80
    resources:
      limits:
        cpu: "1"
        memory: 2Gi
      requests:
        cpu: "1"
        memory: 2Gi
    livenessProbe:
      httpGet:
        path: /health
        port: 80
      initialDelaySeconds: 30
      periodSeconds: 10
    readinessProbe:
      httpGet:
        path: /ready
        port: 80
      initialDelaySeconds: 10
      periodSeconds: 5
```

### 2. Service (if `network.service: true`)

```yaml
apiVersion: v1
kind: Service
metadata:
  name: web-app-service
  labels:
    app: web-app
    managed-by: aether
spec:
  type: ClusterIP  # or NodePort, LoadBalancer
  selector:
    app: web-app
  ports:
  - port: 8080
    targetPort: 80
    protocol: TCP
```

### 3. PersistentVolumeClaim (if `persistence.enabled: true`)

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: web-app-pvc
  labels:
    app: web-app
    managed-by: aether
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: standard
```

When persistence is enabled, the PVC is automatically mounted at `/data` inside the container.

### 4. Volume Mounts (ConfigMaps, Secrets, PVCs)

Aether automatically creates volume mounts when `mount_path` is specified in ConfigMap or Secret definitions, or when persistence is enabled.

**ConfigMap volume mount:**

When a ConfigMap has a `mount_path` field, Aether creates a read-only volume mount:

```yaml
config:
  configMaps:
    - name: app-config
      mount_path: /etc/app          # Mounted as read-only volume
      data:
        app.conf: |
          server.port=8080
          log.level=info
```

This generates a `Volume` + `VolumeMount` pair in the Pod spec:

```yaml
volumes:
  - name: cm-app-config
    configMap:
      name: app-config
containers:
  - volumeMounts:
      - name: cm-app-config
        mountPath: /etc/app
        readOnly: true
```

**Secret volume mount:**

Similarly, Secrets with `mount_path` are mounted as read-only volumes:

```yaml
config:
  secrets:
    - name: tls-certs
      mount_path: /etc/tls          # Mounted as read-only volume
      data:
        tls.crt: "<base64>"
        tls.key: "<base64>"
```

```yaml
volumes:
  - name: secret-tls-certs
    secret:
      secretName: tls-certs
containers:
  - volumeMounts:
      - name: secret-tls-certs
        mountPath: /etc/tls
        readOnly: true
```

**PVC auto-mount:**

When `persistence.enabled: true`, a PVC volume is automatically mounted at `/data`:

```yaml
volumes:
  - name: web-app-storage
    persistentVolumeClaim:
      claimName: web-app-pvc
containers:
  - volumeMounts:
      - name: web-app-storage
        mountPath: /data
```

**Combining multiple volume types:**

All three volume types can be used together in a single workload:

```yaml
config:
  configMaps:
    - name: nginx-conf
      mount_path: /etc/nginx/conf.d
      data:
        default.conf: "server { listen 80; }"
  secrets:
    - name: app-secrets
      mount_path: /run/secrets

persistence:
  enabled: true
  size: 50Gi
```

This creates three volumes: `cm-nginx-conf`, `secret-app-secrets`, and `web-app-storage`.

## Service Types

### ClusterIP (default)

- Internal-only access
- Other pods can reach it via `web-app-service:8080`
- Not accessible from outside cluster

```yaml
network:
  serviceType: ClusterIP
```

### NodePort

- Accessible on each node's IP at a static port
- Port range: 30000-32767
- External access via `<NodeIP>:<NodePort>`

```yaml
network:
  serviceType: NodePort
```

### LoadBalancer

- Cloud provider creates external load balancer
- Automatic public IP assignment
- Works on GKE, EKS, AKS

```yaml
network:
  serviceType: LoadBalancer
```

After deployment, get the external IP:

```bash
kubectl get svc web-app-service
```

## Persistence

### Enable Storage

```yaml
persistence:
  enabled: true
  size: 10Gi
  accessMode: ReadWriteOnce
  storageClass: fast-ssd
```

### Access Modes

- **ReadWriteOnce** - Single node read/write (default)
- **ReadOnlyMany** - Multiple nodes read-only
- **ReadWriteMany** - Multiple nodes read/write (needs NFS/Ceph)

### Storage Classes

Common storage classes by provider:

**Minikube:**
- `standard` (default)

**GKE:**
- `standard` (HDD)
- `standard-rwo` (Persistent Disk)
- `premium-rwo` (SSD Persistent Disk)

**EKS:**
- `gp2` (General Purpose SSD)
- `gp3` (General Purpose SSD v3)
- `io1` (Provisioned IOPS)

**AKS:**
- `default` (Standard HDD)
- `managed-premium` (Premium SSD)

Check available storage classes:

```bash
kubectl get storageclass
```

## Health Checks

Aether automatically configures Kubernetes probes:

### Liveness Probe

- Restarts container if unhealthy
- Detects deadlocks and crashes

```yaml
health:
  liveness:
    httpGet:
      path: /health
      port: 80
    initialDelaySeconds: 30
    periodSeconds: 10
```

### Readiness Probe

- Controls traffic routing
- Removes pod from service if not ready

```yaml
health:
  readiness:
    httpGet:
      path: /ready
      port: 80
    initialDelaySeconds: 10
    periodSeconds: 5
```

## Resource Management

### CPU

- `"1"` = 1 CPU core
- `"500m"` = 0.5 CPU cores
- `"2000m"` = 2 CPU cores

```yaml
requirements:
  cpu: "2"  # or "2000m"
```

### Memory

- `"4Gi"` = 4 gibibytes
- `"4096Mi"` = 4096 mebibytes
- `"4G"` = 4 gigabytes

```yaml
requirements:
  memory: 4Gi
```

Aether sets both `requests` and `limits` to the same value for guaranteed QoS.

## Labels and Annotations

### Automatic Labels

Aether adds these labels to all resources:

- `app: <workload-name>` - Application identifier
- `managed-by: aether` - Management tool

### Custom Labels

Add your own labels:

```yaml
metadata:
  labels:
    env: production
    tier: frontend
    team: platform
```

### Annotations

```yaml
metadata:
  annotations:
    description: "Production web application"
    oncall: "platform-team@example.com"
```

## Multi-Cluster Deployment

### Switch Between Clusters

```bash
# View contexts
kubectl config get-contexts

# Switch to different cluster
kubectl config use-context gke-production

# Deploy to new cluster
aether run --spec workload-k8s.yaml
```

### Namespace Isolation

```bash
# Development (using CLI flag)
aether -n dev run --spec workload-k8s.yaml

# Production (using CLI flag)
aether -n prod run --spec workload-k8s.yaml

# Or using environment variable
export AETHER_NAMESPACE=dev
aether run --spec workload-k8s.yaml
```

## Troubleshooting

### Pod Not Starting

```bash
# Check pod events
kubectl describe pod web-app

# Check logs
kubectl logs web-app

# Common issues:
# - Image pull errors (check registry credentials)
# - Insufficient resources (check node capacity)
# - Health check failures (check probe endpoints)
```

### Service Not Accessible

```bash
# Check service
kubectl get svc web-app-service

# Check endpoints
kubectl get endpoints web-app-service

# Test from inside cluster
kubectl run -it --rm debug --image=alpine --restart=Never -- sh
# Inside pod:
# wget -O- http://web-app-service:8080
```

### Image Pull Errors

```bash
# Create image pull secret
kubectl create secret docker-registry regcred \
  --docker-server=docker.io \
  --docker-username=youruser \
  --docker-password=yourpass \
  --docker-email=your@email.com

# Then reference in workload (future feature)
```

## Advanced Features

### Workload kinds (`kubernetes.workloadKind`)

| Kind | Use case |
|------|----------|
| `deployment` | Default stateless apps |
| `statefulset` | Databases / stable pod identity (uses `volumeClaimTemplates` when persistence enabled) |
| `daemonset` | Node agents |
| `job` | One-off batch runs (`kubernetes.job` settings) |
| `cronJob` | Scheduled jobs (`schedule.cron`) |

Example StatefulSet with image pull secret and TCP startup probe:

```yaml
kubernetes:
  workloadKind: statefulset
  imagePullSecrets:
    - regcred
  serviceAccountName: app
  podDisruptionBudget:
    minAvailable: "1"
  extraVolumes:
    - name: cache
      mountPath: /cache
      volumeType: emptyDir
      volumeConfig: {}
health:
  startup:
    tcpSocket:
      port: 8080
    initialDelaySeconds: 5
    periodSeconds: 5
```

Gateway API (`kubernetes.gateway`), VPA (`kubernetes.verticalPodAutoscaler`), and KEDA (`kubernetes.keda`) generate optional CR manifests when enabled.

### Image pull secrets

```yaml
kubernetes:
  imagePullSecrets:
    - regcred
```

Create the secret with kubectl, then reference it in the workload spec above.

### Port Forwarding (Development)

```bash
# Forward local port to pod
kubectl port-forward web-app 8080:80

# Access at http://localhost:5090
```

### Execute Commands

```bash
# Get shell in pod
kubectl exec -it web-app -- /bin/sh

# Run one-off command
kubectl exec web-app -- ls /app
```

### Copy Files

```bash
# Copy to pod (prefix remote path with pod:)
aether cp web-app ./local-file.txt pod:/tmp/local-file.txt

# Copy from pod
aether cp web-app pod:/app/output.log ./output.log
```

Or use kubectl directly:

```bash
# Copy to pod
kubectl cp local-file.txt web-app:/tmp/

# Copy from pod
kubectl cp web-app:/app/output.log ./output.log
```

## Cleanup

### Delete Specific Workload

```bash
aether delete web-app
```

### Delete All Aether Resources

```bash
kubectl delete all -l managed-by=aether
kubectl delete pvc -l managed-by=aether
```

### Delete Namespace

```bash
kubectl delete namespace aether-demo
```

## Next Steps

- **Multi-cluster GitOps** — extend `AETHER_CONTEXT` per-environment pipelines
- **Live cluster E2E** — enable `AETHER_LABS_LIVE=1` on a reference runner

---

## Example Commands Reference

```bash
# Validate
aether validate --spec workload-k8s.yaml

# Deploy
aether run --spec workload-k8s.yaml --runtime kube

# Status
aether status web-app

# Logs
aether logs web-app --follow

# Delete
aether delete web-app

# List all
aether list

# With custom namespace (CLI flag)
aether -n production run --spec workload-k8s.yaml

# Or using environment variable
AETHER_NAMESPACE=production aether run --spec workload-k8s.yaml
```
