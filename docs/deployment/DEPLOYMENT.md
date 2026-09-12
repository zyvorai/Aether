---
hero:
  eyebrow: DEPLOYMENT
  title: Aether Deployment Guide
  tone: sky
---

This guide covers all deployment options for Aether across different environments and use cases.

## Deployment Options

Aether can be deployed in multiple ways:

1. **Container** - Docker/Podman (recommended for quick start)
2. **Binary** - Direct installation from GitHub releases
3. **Package Manager** - DEB/RPM for Linux distributions
4. **Helm Chart** - Kubernetes operator deployment
5. **From Source** - Build from source code

---

## 1. Container Deployment

### Prerequisites
- Docker or Podman installed

### Quick Start

```bash
# Pull latest image
docker pull ghcr.io/zyvorai/aether:latest

# Create alias for convenience
alias aether='docker run --rm \
  -v ~/.aether:/root/.aether \
  -v ~/.kube:/root/.kube \
  ghcr.io/zyvorai/aether:latest'

# Use normally
aether --help
aether -s workload.yaml validate
```

### Persistent Data

Mount state directory for persistence:

```bash
docker run --rm \
  -v ~/.aether:/root/.aether \
  -v ~/.kube:/root/.kube \
  -v $(pwd):/workspace \
  -w /workspace \
  ghcr.io/zyvorai/aether:latest \
  run -s workload.yaml
```

### Docker Compose

```yaml
version: '3.8'
services:
  aether:
    image: ghcr.io/zyvorai/aether:latest
    volumes:
      - aether-state:/root/.aether
      - ~/.kube:/root/.kube:ro
    command: tail -f /dev/null  # Keep running

volumes:
  aether-state:
```

---

## 2. Binary Installation

### Download from GitHub Releases

```bash
# Linux (amd64)
curl -L https://github.com/zyvorai/Aether/releases/latest/download/aether-linux-amd64 -o aether

# macOS (amd64)
curl -L https://github.com/zyvorai/Aether/releases/latest/download/aether-macos-amd64 -o aether

# macOS (ARM64)
curl -L https://github.com/zyvorai/Aether/releases/latest/download/aether-macos-arm64 -o aether

# Make executable
chmod +x aether

# Move to PATH
sudo mv aether /usr/local/bin/
```

### Verify Installation

```bash
aether --version
aether --help
```

---

## 3. Package Manager Installation

### Debian/Ubuntu (DEB)

#### Download and Install

```bash
# Download package
wget https://github.com/zyvorai/Aether/releases/download/v0.1.0/aether_0.1.0-1_amd64.deb

# Install
sudo apt install ./aether_0.1.0-1_amd64.deb

# Or use dpkg
sudo dpkg -i aether_0.1.0-1_amd64.deb
sudo apt install -f  # Fix dependencies
```

#### Verify Installation

```bash
which aether
dpkg -L aether  # List installed files
```

#### Shell Completions

Automatically installed to:
- Bash: `/usr/share/bash-completion/completions/aether`
- Zsh: `/usr/share/zsh/site-functions/_aether`
- Fish: `/usr/share/fish/vendor_completions.d/aether.fish`

### Fedora/RHEL/CentOS (RPM)

#### Download and Install

```bash
# Download package
wget https://github.com/zyvorai/Aether/releases/download/v0.1.0/aether-0.1.0-1.x86_64.rpm

# Install with DNF (Fedora/RHEL 8+)
sudo dnf install aether-0.1.0-1.x86_64.rpm

# Or with YUM (RHEL 7)
sudo yum install aether-0.1.0-1.x86_64.rpm

# Or with RPM
sudo rpm -ivh aether-0.1.0-1.x86_64.rpm
```

#### Verify Installation

```bash
which aether
rpm -ql aether  # List installed files
```

### openSUSE (RPM)

```bash
# Download package
wget https://github.com/zyvorai/Aether/releases/download/v0.1.0/aether-0.1.0-1.x86_64.rpm

# Install
sudo zypper install aether-0.1.0-1.x86_64.rpm
```

---

## 4. Helm Chart Deployment

### Prerequisites
- Kubernetes 1.20+
- Helm 3.0+
- (Optional) KubeVirt CRDs

### Installation

#### Basic Installation

```bash
# From repository
git clone https://github.com/zyvorai/Aether
cd aether

# Install with default values
helm install aether ./helm/aether
```

#### Custom Values

```bash
# Install with custom configuration
helm install aether ./helm/aether -f - <<EOF
replicaCount: 1

persistence:
  enabled: true
  size: 10Gi

metrics:
  enabled: true
  serviceMonitor:
    enabled: true

aether:
  runtimes:
    kubernetes:
      enabled: true
    kubevirt:
      enabled: true
EOF
```

#### Production Configuration

```bash
helm install aether ./helm/aether \
  --namespace aether-system \
  --create-namespace \
  --set replicaCount=3 \
  --set autoscaling.enabled=true \
  --set autoscaling.minReplicas=2 \
  --set autoscaling.maxReplicas=10 \
  --set persistence.enabled=true \
  --set persistence.size=20Gi \
  --set metrics.serviceMonitor.enabled=true \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=aether.example.com
```

### Verification

```bash
# Check deployment
helm status aether
kubectl get pods -l app.kubernetes.io/name=aether

# View logs
kubectl logs -l app.kubernetes.io/name=aether

# Access metrics
kubectl port-forward service/aether 9090:9090
curl http://localhost:9090/metrics
```

### Upgrade

```bash
helm upgrade aether ./helm/aether -f custom-values.yaml
```

### Uninstall

```bash
helm uninstall aether
kubectl delete pvc -l app.kubernetes.io/instance=aether
```

---

## 5. Build from Source

### Prerequisites
- Rust 1.70+
- Cargo
- Git

### Build Steps

```bash
# Clone repository
git clone https://github.com/zyvorai/Aether
cd aether

# Build release binary
cargo build --release

# Run tests
cargo test --all

# Install
sudo cp target/release/aether /usr/local/bin/

# Generate shell completions
aether completions bash > aether.bash
aether completions zsh > _aether
aether completions fish > aether.fish

# Install completions
sudo cp aether.bash /etc/bash_completion.d/
sudo cp _aether /usr/share/zsh/site-functions/
sudo cp aether.fish /usr/share/fish/vendor_completions.d/
```

---

## Monitoring Setup

### Prometheus Integration

#### Node Exporter Textfile Collector

```bash
# Create metrics export script
sudo tee /usr/local/bin/aether-metrics.sh <<'EOF'
#!/bin/bash
METRICS_DIR="/var/lib/node_exporter/textfile_collector"
mkdir -p "${METRICS_DIR}"
aether metrics > "${METRICS_DIR}/aether.prom.$$"
mv "${METRICS_DIR}/aether.prom.$$" "${METRICS_DIR}/aether.prom"
EOF

sudo chmod +x /usr/local/bin/aether-metrics.sh

# Add to cron (every 5 minutes)
echo "*/5 * * * * /usr/local/bin/aether-metrics.sh" | sudo crontab -
```

#### Prometheus Configuration

```yaml
scrape_configs:
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']
```

### Grafana Dashboard

```bash
# Import dashboard
# 1. Open Grafana
# 2. Navigate to Dashboards → Import
# 3. Upload grafana/dashboard.json
# 4. Select Prometheus datasource
# 5. Import
```

---

## Networking

### Podman Socket

Ensure Podman socket is available:

```bash
systemctl --user enable podman.socket
systemctl --user start podman.socket

# Verify
ls /var/run/podman/podman.sock
```

### Kubernetes Access

Ensure kubeconfig is configured:

```bash
# Verify access
kubectl cluster-info
kubectl get nodes

# Copy to container (if using Docker)
docker run --rm \
  -v ~/.kube:/root/.kube \
  ghcr.io/zyvorai/aether:latest \
  kubectl get nodes
```

---

## Troubleshooting

### Container Issues

**Permission Denied:**
```bash
# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker
```

**Cannot connect to Kubernetes:**
```bash
# Verify kubeconfig
kubectl config view
kubectl cluster-info

# Check kubeconfig path
ls -la ~/.kube/config
```

### Package Issues

**Missing Dependencies (DEB):**
```bash
sudo apt update
sudo apt install -f
```

**Missing Dependencies (RPM):**
```bash
sudo dnf install --allowerasing aether
```

### Helm Issues

**RBAC Errors:**
```bash
# Verify RBAC is enabled
kubectl get clusterrole aether
kubectl get clusterrolebinding aether

# Check service account
kubectl get serviceaccount -n aether-system
```

**Pod Not Starting:**
```bash
kubectl describe pod -l app.kubernetes.io/name=aether
kubectl logs -l app.kubernetes.io/name=aether
```

---

## Best Practices

1. **Use Container for Development**: Quick setup, consistent environment
2. **Use Packages for Servers**: System integration, automatic updates
3. **Use Helm for Kubernetes**: Native integration, easy scaling
4. **Enable Metrics**: Monitor workload operations and performance
5. **Configure Persistence**: Preserve state across restarts
6. **Use RBAC**: Proper permissions for Kubernetes operations
7. **Set Resource Limits**: Prevent resource exhaustion
8. **Enable Autoscaling**: Handle variable load automatically

---

## Next Steps

After deployment:

1. **Validate Installation:**
   ```bash
   aether --version
   aether --help
   ```

2. **Create First Workload:**
   ```bash
   aether -s examples/workload-full-featured.yaml validate
   aether -s examples/workload-full-featured.yaml run
   ```

3. **Monitor Operations:**
   ```bash
   aether list
   aether metrics
   ```

4. **Set Up Monitoring:**
   - Configure Prometheus scraping
   - Import Grafana dashboard
   - Set up alerting rules

5. **Explore Features:**
   - Try migrations between runtimes
   - Use TUI dashboard
   - Configure ConfigMaps and Secrets

---

## Support

- **Documentation**: https://github.com/zyvorai/Aether
- **Issues**: https://github.com/zyvorai/Aether/issues
- **Discussions**: https://github.com/zyvorai/Aether/discussions
