# Aether Packaging

This directory contains packaging specifications for distributing Aether across different Linux distributions.

## Supported Formats

- **DEB** (Debian, Ubuntu, Linux Mint)
- **RPM** (Fedora, RHEL, CentOS, openSUSE)
- **Tarball** (Universal)

## Quick Install

### Debian/Ubuntu

```bash
# Download DEB package
wget https://github.com/ssahani/aether/releases/download/v0.1.0/aether_0.1.0-1_amd64.deb

# Install
sudo dpkg -i aether_0.1.0-1_amd64.deb

# Or use apt
sudo apt install ./aether_0.1.0-1_amd64.deb
```

### Fedora/RHEL/CentOS

```bash
# Download RPM package
wget https://github.com/ssahani/aether/releases/download/v0.1.0/aether-0.1.0-1.x86_64.rpm

# Install
sudo rpm -ivh aether-0.1.0-1.x86_64.rpm

# Or use dnf/yum
sudo dnf install aether-0.1.0-1.x86_64.rpm
```

### openSUSE

```bash
# Download RPM package
wget https://github.com/ssahani/aether/releases/download/v0.1.0/aether-0.1.0-1.x86_64.rpm

# Install
sudo zypper install aether-0.1.0-1.x86_64.rpm
```

## Building Packages

### Prerequisites

#### For DEB (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install build-essential debhelper devscripts cargo rustc
```

#### For RPM (Fedora/RHEL)

```bash
sudo dnf install rpm-build rpmdevtools cargo rust
```

### Build DEB Package

```bash
# From repository root
dpkg-buildpackage -us -uc -b

# Package will be created in parent directory
ls ../aether_0.1.0-1_amd64.deb
```

### Build RPM Package

```bash
# Setup RPM build tree
rpmdev-setuptree

# Copy source
tar czf ~/rpmbuild/SOURCES/aether-0.1.0.tar.gz .

# Build package
rpmbuild -bb rpm/aether.spec

# Package will be in
ls ~/rpmbuild/RPMS/x86_64/aether-0.1.0-1.*.x86_64.rpm
```

## Package Contents

Both DEB and RPM packages include:

### Binaries
- `/usr/bin/aether` - Main executable

### Data Files
- `/usr/share/aether/schema/workload.schema.json` - JSON Schema

### Shell Completions
- `/usr/share/bash-completion/completions/aether` - Bash
- `/usr/share/zsh/site-functions/_aether` - Zsh
- `/usr/share/fish/vendor_completions.d/aether.fish` - Fish

### Documentation
- `/usr/share/doc/aether/README.md`
- `/usr/share/doc/aether/CHANGELOG.md`
- `/usr/share/doc/aether/METRICS.md`
- `/usr/share/doc/aether/SCHEMA.md`
- `/usr/share/doc/aether/examples/`

## Continuous Integration

Packages are automatically built on releases via GitHub Actions:

### Workflow: `.github/workflows/package.yml`

Builds:
- DEB for Ubuntu 22.04 (amd64, arm64)
- RPM for Fedora 39 (x86_64, aarch64)

Artifacts are attached to GitHub releases.

## Repository Setup

### APT Repository (Debian/Ubuntu)

```bash
# Add GPG key
wget -O- https://packagecloud.io/aether/stable/gpgkey | sudo apt-key add -

# Add repository
echo "deb https://packagecloud.io/aether/stable/ubuntu/ jammy main" | \
  sudo tee /etc/apt/sources.list.d/aether.list

# Update and install
sudo apt update
sudo apt install aether
```

### YUM/DNF Repository (Fedora/RHEL)

```bash
# Add repository
sudo tee /etc/yum.repos.d/aether.repo <<EOF
[aether]
name=Aether Repository
baseurl=https://packagecloud.io/aether/stable/rpm_any/rpm_any/\$basearch
repo_gpgcheck=1
gpgcheck=0
enabled=1
gpgkey=https://packagecloud.io/aether/stable/gpgkey
sslverify=1
sslcacert=/etc/pki/tls/certs/ca-bundle.crt
metadata_expire=300
EOF

# Install
sudo dnf install aether
```

## Verification

### Verify Package Integrity

#### DEB
```bash
dpkg -c aether_0.1.0-1_amd64.deb  # List contents
dpkg -I aether_0.1.0-1_amd64.deb  # Show info
```

#### RPM
```bash
rpm -qpl aether-0.1.0-1.x86_64.rpm  # List contents
rpm -qpi aether-0.1.0-1.x86_64.rpm  # Show info
```

### Verify Installation

```bash
# Check binary
which aether
aether --version

# Check completions
ls /usr/share/bash-completion/completions/aether

# Check documentation
ls /usr/share/doc/aether/
```

## Uninstallation

### Debian/Ubuntu

```bash
sudo apt remove aether
# or
sudo dpkg -r aether
```

### Fedora/RHEL

```bash
sudo dnf remove aether
# or
sudo rpm -e aether
```

## Troubleshooting

### Missing Dependencies (DEB)

```bash
sudo apt install -f
```

### Missing Dependencies (RPM)

```bash
sudo dnf install --allowerasing aether
```

### Permission Issues

Ensure you have sudo/root access for installation:
```bash
sudo -v
```

## Contributing

To improve packaging:

1. Test on target distribution
2. Update `debian/control` or `rpm/aether.spec`
3. Rebuild and test package
4. Submit pull request

## Package Hosting

Packages are hosted on:
- GitHub Releases: https://github.com/ssahani/aether/releases
- PackageCloud (optional): https://packagecloud.io/aether/stable

## Support

For packaging issues:
- GitHub Issues: https://github.com/ssahani/aether/issues
- Tag: `packaging`

## References

- [Debian New Maintainer's Guide](https://www.debian.org/doc/manuals/maint-guide/)
- [RPM Packaging Guide](https://rpm-packaging-guide.github.io/)
- [Debian Policy Manual](https://www.debian.org/doc/debian-policy/)
- [Fedora Packaging Guidelines](https://docs.fedoraproject.org/en-US/packaging-guidelines/)
