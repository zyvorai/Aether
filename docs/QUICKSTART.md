# Aether Quickstart

## Prerequisites

- Kubernetes 1.28+ (for cluster/VM/metal targets)
- Podman 4+ (for container target)
- `helm` 3.8+
- `kubectl` configured

## 1. Install Aether (Helm)

```bash
helm install aether oci://ghcr.io/hypersdk/charts/aether \
  --version 0.1.0 \
  --namespace aether-system \
  --create-namespace
```

## 2. Install CLI

```bash
curl -LO https://releases.zyvor.dev/aether/latest/aether-linux-x86_64
chmod +x aether-linux-x86_64 && sudo mv aether-linux-x86_64 /usr/local/bin/aether
aether --version
```

## 3. Verify

```bash
kubectl -n aether-system rollout status deployment/aether
aether status
```

## 4. Deploy Your First Workload

```bash
# Create a workload spec
cat > workload.yaml <<EOF
apiVersion: aether.zyvor.dev/v1
kind: Workload
metadata:
  name: my-app
spec:
  image: nginx:latest
  replicas: 2
  runtime: kubernetes
  resources:
    cpu: "250m"
    memory: "256Mi"
  ports:
    - containerPort: 80
      service: true
EOF

# Deploy
aether run -f workload.yaml

# Check status
aether status my-app

# View in dashboard
aether dashboard   # opens http://localhost:9090
```

## 5. Migrate Between Runtimes

```bash
# Move from Kubernetes to KubeVirt (no downtime)
aether migrate my-app --to kubevirt --strategy blue-green

# Check progress
aether migrate status my-app
```
