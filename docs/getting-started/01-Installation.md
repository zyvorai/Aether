---
hero:
  eyebrow: GETTING STARTED
  title: 🛠️ Installation Guide
  tone: emerald
  lead: Install aether -- the Universal Runtime Control Plane -- from source, packages,
    containers, or Helm.
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

> **Tip:** For local development, start with Podman only. You can add Kubernetes
> and other runtimes later without reinstalling aether.

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
git clone https://github.com/zyvorai/Aether.git
cd aether

# 2. Build the release binary (optimized)
cargo build --release

# 3. Install to your PATH
sudo install -m 0755 target/release/aether /usr/local/bin/aether

# 4. Verify
aether --version
```

**Expected output:**

```
aether 0.3.0
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
wget https://github.com/zyvorai/Aether/releases/latest/download/aether_0.3.0-1_amd64.deb

# Install
sudo apt install ./aether_0.3.0-1_amd64.deb

# Verify
aether --version
```

### Fedora / RHEL / CentOS (.rpm)

```bash
# Download the latest release
wget https://github.com/zyvorai/Aether/releases/latest/download/aether-0.3.0-1.x86_64.rpm

# Install
sudo dnf install ./aether-0.3.0-1.x86_64.rpm

# Verify
aether --version
```

### Arch Linux (AUR)

```bash
# Via yay or paru
yay -S aether
```

---

## 📥 Option 3: Cargo Install

If you have the Rust toolchain installed, you can install directly from crates.io or from the repository:

```bash
# From the local repository
cargo install --path .

# Or install from a git URL
cargo install --git https://github.com/zyvorai/Aether.git

# Verify
aether --version
```

> **Note:** `cargo install` places the binary in `~/.cargo/bin/`. Make sure this
> directory is in your `$PATH`.

---

## 🐳 Option 4: Container Image

Run aether as a container without installing anything on the host:

```bash
# Pull from GitHub Container Registry
podman pull ghcr.io/zyvorai/aether:latest

# Or with Docker
docker pull ghcr.io/zyvorai/aether:latest
```

### Create a Shell Alias

Add this to your `~/.bashrc` or `~/.zshrc`:

```bash
alias aether='podman run --rm -it \
  -v ~/.aether:/root/.aether \
  -v ~/.kube:/root/.kube:ro \
  -v /run/podman/podman.sock:/run/podman/podman.sock \
  -v "$(pwd):/work" -w /work \
  ghcr.io/zyvorai/aether:latest'
```

### Verify

```bash
source ~/.bashrc
aether --version
```

> **Note:** When running in a container, aether needs access to the host's
> runtime sockets (e.g., Podman socket) and kubeconfig. The volume mounts above
> handle the common cases.

---

## ⎈ Option 5: Helm Chart (Kubernetes)

Deploy aether as a Kubernetes service with the API server and web dashboard:

```bash
# From OCI (when charts are published to GHCR)
helm install aether oci://ghcr.io/zyvorai/charts/aether --version 0.4.0

# Or from a local checkout of this repository
helm install aether ./helm/aether
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
helm install aether ./helm/aether -f values.yaml
```

### Verify Helm Deployment

```bash
kubectl get pods -l app=aether
kubectl port-forward svc/aether 8080:8080
curl http://localhost:5090/health
```

---

## 🐚 Shell Completions

Enable tab completion for faster command entry. Aether supports five shells:

### Bash

```bash
# System-wide (requires root)
aether completions bash | sudo tee /etc/bash_completion.d/aether > /dev/null

# User-only
mkdir -p ~/.local/share/bash-completion/completions
aether completions bash > ~/.local/share/bash-completion/completions/aether

# Activate in current session
source <(aether completions bash)
```

### Zsh

```bash
# Create completions directory if needed
mkdir -p ~/.zsh/completion

# Generate completions
aether completions zsh > ~/.zsh/completion/_aether

# Add to .zshrc (if not already present)
echo 'fpath=(~/.zsh/completion $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Reload
source ~/.zshrc
```

### Fish

```bash
# Generate and install
aether completions fish > ~/.config/fish/completions/aether.fish

# Activate immediately
source ~/.config/fish/completions/aether.fish
```

### PowerShell

```powershell
# Generate completions
aether completions powershell > $HOME\.config\aether.ps1

# Add to PowerShell profile
echo '. $HOME\.config\aether.ps1' >> $PROFILE
```

### Elvish

```bash
aether completions elvish > ~/.elvish/lib/aether.elv
```

### Test Completions

After installing, test by typing:

```bash
aether <TAB><TAB>
```

You should see all available subcommands listed.

---

## 🧙 First-Time Setup Wizard

After installing aether, run the setup wizard to configure your environment:

```bash
aether init
```

The wizard will:

1. **Detect available runtimes** -- scans for `podman`, `kubectl`, `virtctl` on your `$PATH`
2. **Create the state directory** -- initializes `~/.aether/` with default configuration
3. **Generate a sample workload** -- creates `workload.yaml` in the current directory
4. **Show next steps** -- recommendations based on your detected runtimes

### Manual Configuration

If you prefer to configure manually:

```bash
# Initialize configuration only
aether config --init

# View current configuration
aether config --show
```

### Configuration Location

| File | Purpose |
|------|---------|
| `~/.aether/config.yaml` | Global configuration |
| `~/.aether/state.json` | Workload state database |
| `~/.aether/audit.json` | Audit trail |
| `~/.aether/plugins/` | Plugin manifest directory |
| `~/.aether/secrets.json` | Encrypted secrets store |
| `~/.aether/health.json` | Health history records |

---

## ✅ Verify Installation

Run through this checklist to confirm everything is working:

```bash
# 1. Check the binary version
aether --version
# Expected: aether 0.3.0

# 2. Run the setup wizard
aether init

# 3. Validate the generated sample workload
aether validate
# Expected: Validation passed

# 4. View all available commands
aether help-all

# 5. Check runtime detection
aether recommend
# Shows AI-scored runtime recommendations for workload.yaml

# 6. Test JSON output mode
aether list --output json
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
```

---

## 🔧 Troubleshooting

### "command not found: aether"

Ensure the binary is in your `$PATH`:

```bash
# Check where it was installed
which aether

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
| Explore all commands | `aether help-all` |
| Full documentation index | [Documentation Index](../index.md) |
| Documentation hub | [README](https://github.com/zyvorai/Aether/blob/main/docs/README.md) |
