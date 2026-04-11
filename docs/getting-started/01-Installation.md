# 🛠️ Installation Guide

> Install orchestr8 -- the Universal Runtime Control Plane -- from source, packages, containers, or Helm.

---

## 📖 Table of Contents

- [Prerequisites](#-prerequisites)
- [Option 1: Build from Source (Recommended)](#-option-1-build-from-source-recommended)
- [Option 2: Package Managers](#-option-2-package-managers)
- [Option 3: Cargo Install](#-option-3-cargo-install)
- [Option 4: Container Image](#-option-4-container-image)
- [Option 5: Helm Chart (Kubernetes)](#-option-5-helm-chart-kubernetes)
- [Shell Completions](#-shell-completions)
- [First-Time Setup Wizard](#-first-time-setup-wizard)
- [Verify Installation](#-verify-installation)
- [Troubleshooting](#-troubleshooting)
- [Next Steps](#-next-steps)

---

## 📋 Prerequisites

### System Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| **Rust toolchain** | 1.70+ | Latest stable (rustup) |
| **Operating System** | Linux x86_64, aarch64 | Fedora 39+, Ubuntu 22.04+, RHEL 9+ |
| **Disk space** | 100 MB (binary) | 500 MB (build + state) |
| **Memory** | 256 MB | 512 MB |

### Runtime Dependencies

Install the runtimes you plan to target. You only need the ones you will use:

| Runtime | Required Tools | Installation |
|---------|---------------|--------------|
| 🐳 **Podman** (container) | `podman` | `sudo dnf install podman` or `sudo apt install podman` |
| ☸️ **Kubernetes** | `kubectl` | [kubernetes.io/docs/tasks/tools](https://kubernetes.io/docs/tasks/tools/) |
| 🖥️ **KubeVirt** | `kubectl` + `virtctl` | [kubevirt.io/user-guide](https://kubevirt.io/user-guide/) |
| 🔩 **Metal3** | `kubectl` | [metal3.io/documentation](https://metal3.io/) |

> **Tip:** For local development, start with Podman only. You can add Kubernetes
> and other runtimes later without reinstalling orchestr8.

### Verify Prerequisites

```bash
# Check Rust version
rustc --version    # Should be 1.70+

# Check available runtimes
which podman       # Container runtime
which kubectl      # Kubernetes CLI
which virtctl      # KubeVirt CLI (optional)
```

---

## 🔨 Option 1: Build from Source (Recommended)

Building from source gives you the latest features and lets you customize the build.

```bash
# 1. Clone the repository
git clone https://github.com/ssahani/orchestr8.git
cd orchestr8

# 2. Build the release binary (optimized)
cargo build --release

# 3. Install to your PATH
sudo install -m 0755 target/release/orchestr8 /usr/local/bin/orchestr8

# 4. Verify
orchestr8 --version
```

**Expected output:**

```
orchestr8 0.3.0
```

### Build Options

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Build with specific features
cargo build --release --features full

# Run tests before installing
cargo test --release
```

### Build from a Specific Tag

```bash
git checkout v0.3.0
cargo build --release
```

---

## 📦 Option 2: Package Managers

### Debian / Ubuntu (.deb)

```bash
# Download the latest release
wget https://github.com/ssahani/orchestr8/releases/latest/download/orchestr8_0.3.0-1_amd64.deb

# Install
sudo apt install ./orchestr8_0.3.0-1_amd64.deb

# Verify
orchestr8 --version
```

### Fedora / RHEL / CentOS (.rpm)

```bash
# Download the latest release
wget https://github.com/ssahani/orchestr8/releases/latest/download/orchestr8-0.3.0-1.x86_64.rpm

# Install
sudo dnf install ./orchestr8-0.3.0-1.x86_64.rpm

# Verify
orchestr8 --version
```

### Arch Linux (AUR)

```bash
# Via yay or paru
yay -S orchestr8
```

---

## 📥 Option 3: Cargo Install

If you have the Rust toolchain installed, you can install directly from crates.io or from the repository:

```bash
# From the local repository
cargo install --path .

# Or install from a git URL
cargo install --git https://github.com/ssahani/orchestr8.git

# Verify
orchestr8 --version
```

> **Note:** `cargo install` places the binary in `~/.cargo/bin/`. Make sure this
> directory is in your `$PATH`.

---

## 🐳 Option 4: Container Image

Run orchestr8 as a container without installing anything on the host:

```bash
# Pull from GitHub Container Registry
podman pull ghcr.io/ssahani/orchestr8:latest

# Or with Docker
docker pull ghcr.io/ssahani/orchestr8:latest
```

### Create a Shell Alias

Add this to your `~/.bashrc` or `~/.zshrc`:

```bash
alias orchestr8='podman run --rm -it \
  -v ~/.orchestr8:/root/.orchestr8 \
  -v ~/.kube:/root/.kube:ro \
  -v /run/podman/podman.sock:/run/podman/podman.sock \
  -v "$(pwd):/work" -w /work \
  ghcr.io/ssahani/orchestr8:latest'
```

### Verify

```bash
source ~/.bashrc
orchestr8 --version
```

> **Note:** When running in a container, orchestr8 needs access to the host's
> runtime sockets (e.g., Podman socket) and kubeconfig. The volume mounts above
> handle the common cases.

---

## ⎈ Option 5: Helm Chart (Kubernetes)

Deploy orchestr8 as a Kubernetes service with the API server and web dashboard:

```bash
# Add the Helm repository
helm repo add orchestr8 https://ssahani.github.io/orchestr8/charts
helm repo update

# Install with default values
helm install orchestr8 orchestr8/orchestr8

# Or install from a local chart directory
helm install orchestr8 ./helm/orchestr8
```

### Custom Values

Create a `values.yaml`:

```yaml
replicaCount: 1

service:
  type: ClusterIP
  port: 8080

api:
  host: "0.0.0.0"
  port: 8080

resources:
  limits:
    cpu: "500m"
    memory: "256Mi"
  requests:
    cpu: "100m"
    memory: "128Mi"
```

```bash
helm install orchestr8 orchestr8/orchestr8 -f values.yaml
```

### Verify Helm Deployment

```bash
kubectl get pods -l app=orchestr8
kubectl port-forward svc/orchestr8 8080:8080
curl http://localhost:8080/health
```

---

## 🐚 Shell Completions

Enable tab completion for faster command entry. Orchestr8 supports five shells:

### Bash

```bash
# System-wide (requires root)
orchestr8 completions bash | sudo tee /etc/bash_completion.d/orchestr8 > /dev/null

# User-only
mkdir -p ~/.local/share/bash-completion/completions
orchestr8 completions bash > ~/.local/share/bash-completion/completions/orchestr8

# Activate in current session
source <(orchestr8 completions bash)
```

### Zsh

```bash
# Create completions directory if needed
mkdir -p ~/.zsh/completion

# Generate completions
orchestr8 completions zsh > ~/.zsh/completion/_orchestr8

# Add to .zshrc (if not already present)
echo 'fpath=(~/.zsh/completion $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Reload
source ~/.zshrc
```

### Fish

```bash
# Generate and install
orchestr8 completions fish > ~/.config/fish/completions/orchestr8.fish

# Activate immediately
source ~/.config/fish/completions/orchestr8.fish
```

### PowerShell

```powershell
# Generate completions
orchestr8 completions powershell > $HOME\.config\orchestr8.ps1

# Add to PowerShell profile
echo '. $HOME\.config\orchestr8.ps1' >> $PROFILE
```

### Elvish

```bash
orchestr8 completions elvish > ~/.elvish/lib/orchestr8.elv
```

### Test Completions

After installing, test by typing:

```bash
orchestr8 <TAB><TAB>
```

You should see all available subcommands listed.

---

## 🧙 First-Time Setup Wizard

After installing orchestr8, run the setup wizard to configure your environment:

```bash
orchestr8 init
```

The wizard will:

1. **Detect available runtimes** -- scans for `podman`, `kubectl`, `virtctl` on your `$PATH`
2. **Create the state directory** -- initializes `~/.orchestr8/` with default configuration
3. **Generate a sample workload** -- creates `workload.yaml` in the current directory
4. **Show next steps** -- recommendations based on your detected runtimes

### Manual Configuration

If you prefer to configure manually:

```bash
# Initialize configuration only
orchestr8 config --init

# View current configuration
orchestr8 config --show
```

### Configuration Location

| File | Purpose |
|------|---------|
| `~/.orchestr8/config.yaml` | Global configuration |
| `~/.orchestr8/state.json` | Workload state database |
| `~/.orchestr8/audit.json` | Audit trail |
| `~/.orchestr8/plugins/` | Plugin manifest directory |
| `~/.orchestr8/secrets.json` | Encrypted secrets store |
| `~/.orchestr8/health.json` | Health history records |

---

## ✅ Verify Installation

Run through this checklist to confirm everything is working:

```bash
# 1. Check the binary version
orchestr8 --version
# Expected: orchestr8 0.3.0

# 2. Run the setup wizard
orchestr8 init

# 3. Validate the generated sample workload
orchestr8 validate
# Expected: Validation passed

# 4. View all available commands
orchestr8 help-all

# 5. Check runtime detection
orchestr8 recommend
# Shows AI-scored runtime recommendations for workload.yaml

# 6. Test JSON output mode
orchestr8 list --output json
# Expected: JSON array (empty if no workloads deployed)
```

### Verify Runtime Connectivity

```bash
# Podman
podman version
podman info

# Kubernetes
kubectl cluster-info
kubectl get nodes

# KubeVirt (if installed)
kubectl get kubevirt -n kubevirt

# Metal3 (if installed)
kubectl get baremetalhosts -A
```

---

## 🔧 Troubleshooting

### "command not found: orchestr8"

Ensure the binary is in your `$PATH`:

```bash
# Check where it was installed
which orchestr8

# If using cargo install, add cargo bin to PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

### "Permission denied" on Podman socket

```bash
# Enable the Podman socket for rootless access
systemctl --user enable --now podman.socket

# Verify
podman info
```

### Build fails with missing dependencies

```bash
# Install build essentials
sudo dnf install gcc openssl-devel pkg-config   # Fedora/RHEL
sudo apt install build-essential libssl-dev pkg-config  # Debian/Ubuntu
```

### Kubernetes connection refused

```bash
# Verify kubeconfig
echo $KUBECONFIG
kubectl cluster-info

# If using minikube
minikube start
```

---

## 🔗 Next Steps

| Next | Link |
|------|------|
| Deploy your first workload | [Quick Start Guide](02-Quick-Start.md) |
| Detailed walkthrough tutorial | [Beginner Tutorial](../tutorials/01-beginner-deployment.md) |
| Explore all commands | `orchestr8 help-all` |
| Full documentation index | [Documentation Index](../index.md) |
| Documentation hub | [README](../README.md) |
