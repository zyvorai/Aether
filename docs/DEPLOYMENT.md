# Orchestr8 Deployment Guide

This guide covers all deployment options for Orchestr8across different environments and use cases.

## Deployment Options

Orchestr8 can be deployed in multiple ways:

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
docker pull ghcr.io/ssahani/orchestr8:latest

# Create alias for convenience
alias orchestr8='docker run --rm \
  -v ~/.orchestr8:/root/.orchestr8 \
  -v ~/.kube:/root/.kube \
  ghcr.io/ssahani/orchestr8:latest'

# Use normally
orchestr8 --help
orchestr8 -s workload.yaml validate
```

### Persistent Data

Mount state directory for persistence:

```bash
docker run --rm \
  -v ~/.orchestr8:/root/.orchestr8 \
  -v ~/.kube:/root/.kube \
  -v $(pwd):/workspace \
  -w /workspace \
  ghcr.io/ssahani/orchestr8:latest \
  run -s workload.yaml
```

### Docker Compose

```yaml
version: '3.8'
services:
  orchestr8:
    image: ghcr.io/ssahani/orchestr8:latest
    volumes:
      - orchestr8-state:/root/.orchestr8
      - ~/.kube:/root/.kube:ro
    command: tail -f /dev/null  # Keep running

volumes:
  orchestr8-state:
```

---

## 2. Binary Installation

### Download from GitHub Releases

```bash
# Linux (amd64)
curl -L https://github.com/ssahani/orchestr8/releases/latest/download/orchestr8-linux-amd64 -o orchestr8

# macOS (amd64)
curl -L https://github.com/ssahani/orchestr8/releases/latest/download/orchestr8-macos-amd64 -o orchestr8

# macOS (ARM64)
curl -L https://github.com/ssahani/orchestr8/releases/latest/download/orchestr8-macos-arm64 -o orchestr8

# Make executable
chmod +x orchestr8

# Move to PATH
sudo mv orchestr8 /usr/local/bin/
```

### Verify Installation

```bash
orchestr8 --version
orchestr8 --help
```

---

## 3. Package Manager Installation

### Debian/Ubuntu (DEB)

#### Download and Install

```bash
# Download package
wget https://github.com/ssahani/orchestr8/releases/download/v0.1.0/orchestr8_0.1.0-1_amd64.deb

# Install
sudo apt install ./orchestr8_0.1.0-1_amd64.deb

# Or use dpkg
sudo dpkg -i orchestr8_0.1.0-1_amd64.deb
sudo apt install -f  # Fix dependencies
```

#### Verify Installation

```bash
which orchestr8
dpkg -L orchestr8  # List installed files
```

#### Shell Completions

Automatically installed to:
- Bash: `/usr/share/bash-completion/completions/orchestr8`
- Zsh: `/usr/share/zsh/site-functions/_orchestr8`
- Fish: `/usr/share/fish/vendor_completions.d/orchestr8.fish`

### Fedora/RHEL/CentOS (RPM)

#### Download and Install

```bash
# Download package
wget https://github.com/ssahani/orchestr8/releases/download/v0.1.0/orchestr8-0.1.0-1.x86_64.rpm

# Install with DNF (Fedora/RHEL 8+)
sudo dnf install orchestr8-0.1.0-1.x86_64.rpm

# Or with YUM (RHEL 7)
sudo yum install orchestr8-0.1.0-1.x86_64.rpm

# Or with RPM
sudo rpm -ivh orchestr8-0.1.0-1.x86_64.rpm
```

#### Verify Installation

```bash
which orchestr8
rpm -ql orchestr8  # List installed files
```

### openSUSE (RPM)

```bash
# Download package
wget https://github.com/ssahani/orchestr8/releases/download/v0.1.0/orchestr8-0.1.0-1.x86_64.rpm

# Install
sudo zypper install orchestr8-0.1.0-1.x86_64.rpm
```

---

## 4. Helm Chart Deployment

### Prerequisites
- Kubernetes 1.20+
- Helm 3.0+
- (Optional) KubeVirt CRDs
- (Optional) Metal3 CRDs

### Installation

#### Basic Installation

```bash
# From repository
git clone https://github.com/ssahani/orchestr8
cd orchestr8

# Install with default values
helm install orchestr8 ./helm/orchestr8
```

#### Custom Values

```bash
# Install with custom configuration
helm install orchestr8 ./helm/orchestr8 -f - <<EOF
replicaCount: 1

persistence:
  enabled: true
  size: 10Gi

metrics:
  enabled: true
  serviceMonitor:
    enabled: true

orchestr8:
  runtimes:
    kubernetes:
      enabled: true
    kubevirt:
      enabled: true
EOF
```

#### Production Configuration

```bash
helm install orchestr8 ./helm/orchestr8 \
  --namespace orchestr8-system \
  --create-namespace \
  --set replicaCount=3 \
  --set autoscaling.enabled=true \
  --set autoscaling.minReplicas=2 \
  --set autoscaling.maxReplicas=10 \
  --set persistence.enabled=true \
  --set persistence.size=20Gi \
  --set metrics.serviceMonitor.enabled=true \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=orchestr8.example.com
```

### Verification

```bash
# Check deployment
helm status orchestr8
kubectl get pods -l app.kubernetes.io/name=orchestr8

# View logs
kubectl logs -l app.kubernetes.io/name=orchestr8

# Access metrics
kubectl port-forward service/orchestr8 9090:9090
curl http://localhost:9090/metrics
```

### Upgrade

```bash
helm upgrade orchestr8 ./helm/orchestr8 -f custom-values.yaml
```

### Uninstall

```bash
helm uninstall orchestr8
kubectl delete pvc -l app.kubernetes.io/instance=orchestr8
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
git clone https://github.com/ssahani/orchestr8
cd orchestr8

# Build release binary
cargo build --release

# Run tests
cargo test --all

# Install
sudo cp target/release/orchestr8 /usr/local/bin/

# Generate shell completions
orchestr8 completions bash > orchestr8.bash
orchestr8 completions zsh > _orchestr8
orchestr8 completions fish > orchestr8.fish

# Install completions
sudo cp orchestr8.bash /etc/bash_completion.d/
sudo cp _orchestr8 /usr/share/zsh/site-functions/
sudo cp orchestr8.fish /usr/share/fish/vendor_completions.d/
```

---

## Monitoring Setup

### Prometheus Integration

#### Node Exporter Textfile Collector

```bash
# Create metrics export script
sudo tee /usr/local/bin/orchestr8-metrics.sh <<'EOF'
#!/bin/bash
METRICS_DIR="/var/lib/node_exporter/textfile_collector"
mkdir -p "${METRICS_DIR}"
orchestr8 metrics > "${METRICS_DIR}/orchestr8.prom.$$"
mv "${METRICS_DIR}/orchestr8.prom.$$" "${METRICS_DIR}/orchestr8.prom"
EOF

sudo chmod +x /usr/local/bin/orchestr8-metrics.sh

# Add to cron (every 5 minutes)
echo "*/5 * * * * /usr/local/bin/orchestr8-metrics.sh" | sudo crontab -
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
  ghcr.io/ssahani/orchestr8:latest \
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
sudo dnf install --allowerasing orchestr8
```

### Helm Issues

**RBAC Errors:**
```bash
# Verify RBAC is enabled
kubectl get clusterrole orchestr8
kubectl get clusterrolebinding orchestr8

# Check service account
kubectl get serviceaccount -n orchestr8-system
```

**Pod Not Starting:**
```bash
kubectl describe pod -l app.kubernetes.io/name=orchestr8
kubectl logs -l app.kubernetes.io/name=orchestr8
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
   orchestr8 --version
   orchestr8 --help
   ```

2. **Create First Workload:**
   ```bash
   orchestr8 -s examples/workload-full-featured.yaml validate
   orchestr8 -s examples/workload-full-featured.yaml run
   ```

3. **Monitor Operations:**
   ```bash
   orchestr8 list
   orchestr8 metrics
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

- **Documentation**: https://github.com/ssahani/orchestr8
- **Issues**: https://github.com/ssahani/orchestr8/issues
- **Discussions**: https://github.com/ssahani/orchestr8/discussions
