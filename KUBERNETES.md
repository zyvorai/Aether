# Kubernetes Runtime Guide

This guide explains how to use Orchestr8 with Kubernetes clusters.

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

By default, Orchestr8 uses the `default` namespace. To use a different namespace:

```bash
export ORCHESTR8_NAMESPACE=my-namespace
```

Or create a custom namespace:

```bash
kubectl create namespace orchestr8-demo
export ORCHESTR8_NAMESPACE=orchestr8-demo
```

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
apiVersion: orchestr8/v1
kind: Workload

metadata:
  name: web-app
  owner: yourname
  project: demo

build:
  registry: docker.io/yourorg/orchestr8

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
orchestr8 validate --spec workload-k8s.yaml

# Deploy to cluster
orchestr8 run --spec workload-k8s.yaml

# Or explicitly select Kubernetes runtime
orchestr8 run --spec workload-k8s.yaml --runtime kube
```

### 5. Check Status

```bash
# Get workload status
orchestr8 status web-app

# Or use kubectl directly
kubectl get pods -l app=web-app
kubectl get svc web-app-service
```

### 6. View Logs

```bash
# Stream logs
orchestr8 logs web-app --follow

# Or use kubectl
kubectl logs -l app=web-app -f
```

### 7. Delete Resources

```bash
# Clean up all resources (Pod, Service, PVC)
orchestr8 delete web-app

# Verify deletion
kubectl get all -l managed-by=orchestr8
```

## What Orchestr8 Creates

When you deploy a workload to Kubernetes, Orchestr8 automatically creates:

### 1. Pod

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: web-app
  labels:
    app: web-app
    managed-by: orchestr8
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
    managed-by: orchestr8
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
    managed-by: orchestr8
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 5Gi
  storageClassName: standard
```

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

Orchestr8 automatically configures Kubernetes probes:

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

Orchestr8 sets both `requests` and `limits` to the same value for guaranteed QoS.

## Labels and Annotations

### Automatic Labels

Orchestr8 adds these labels to all resources:

- `app: <workload-name>` - Application identifier
- `managed-by: orchestr8` - Management tool

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
orchestr8 run --spec workload-k8s.yaml
```

### Namespace Isolation

```bash
# Development
export ORCHESTR8_NAMESPACE=dev
orchestr8 run --spec workload-k8s.yaml

# Production
export ORCHESTR8_NAMESPACE=prod
orchestr8 run --spec workload-k8s.yaml
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

### Port Forwarding (Development)

```bash
# Forward local port to pod
kubectl port-forward web-app 8080:80

# Access at http://localhost:8080
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
# Copy to pod
kubectl cp local-file.txt web-app:/tmp/

# Copy from pod
kubectl cp web-app:/app/output.log ./output.log
```

## Cleanup

### Delete Specific Workload

```bash
orchestr8 delete web-app
```

### Delete All Orchestr8 Resources

```bash
kubectl delete all -l managed-by=orchestr8
kubectl delete pvc -l managed-by=orchestr8
```

### Delete Namespace

```bash
kubectl delete namespace orchestr8-demo
```

## Next Steps

- **Horizontal Pod Autoscaler** - Auto-scale based on CPU/memory
- **Ingress** - HTTP/HTTPS routing
- **ConfigMaps & Secrets** - Configuration management
- **StatefulSets** - For databases and stateful apps
- **Jobs & CronJobs** - Batch workloads

---

## Example Commands Reference

```bash
# Validate
orchestr8 validate --spec workload-k8s.yaml

# Deploy
orchestr8 run --spec workload-k8s.yaml --runtime kube

# Status
orchestr8 status web-app

# Logs
orchestr8 logs web-app --follow

# Delete
orchestr8 delete web-app

# List all
orchestr8 list

# With custom namespace
ORCHESTR8_NAMESPACE=production orchestr8 run --spec workload-k8s.yaml
```
