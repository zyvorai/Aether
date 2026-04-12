# 🖧 Metal3 Bare Metal Deployment Guide

Complete guide for provisioning bare metal servers using Aether's Metal3 runtime.

---

## 📋 Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Advanced Features](#advanced-features)
- [Operations](#operations)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

---

## Overview

### What is Metal3?

**Metal3** (Metal Kubed) is a Kubernetes-native bare metal provisioning system. It provides:

- **Bare Metal Management**: Provision physical servers as K8s resources
- **BMC Integration**: IPMI, Redfish, iDRAC support
- **Unified Platform**: Manage containers, VMs, and bare metal together
- **Cloud-Native**: Native Kubernetes APIs and workflows

### Why Use Aether with Metal3?

Aether simplifies bare metal provisioning:

✅ **One Spec Format**: Same YAML for containers, VMs, and bare metal
✅ **Auto-Generation**: Creates BareMetalHost CRDs
✅ **Hardware Matching**: Automatic server selection based on requirements
✅ **Lifecycle Management**: Power on, off, deprovision servers
✅ **Monitoring**: Integrated TUI dashboard

---

## Prerequisites

### Required Components

1. **Kubernetes Cluster** (v1.24+)
   ```bash
   kubectl version --short
   ```

2. **Metal3 Installed** (v0.7+)
   ```bash
   kubectl get baremetalhosts -A
   ```

3. **Bare Metal Inventory API (Ironic)**
   ```bash
   kubectl get pods -n baremetal-operator-system
   ```

4. **BMC Network Access**
   - Network connectivity to server BMCs (IPMI/Redfish)
   - BMC credentials configured

### Install Metal3 (if not present)

```bash
# Set version
export METAL3_VERSION=v1.5.0

# Install cert-manager (required)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Wait for cert-manager
kubectl wait --for=condition=Available --timeout=300s \
  deployment/cert-manager -n cert-manager

# Install baremetal-operator
kubectl apply -f https://github.com/metal3-io/baremetal-operator/releases/download/${METAL3_VERSION}/baremetal-operator.yaml

# Wait for deployment
kubectl wait --for=condition=Available --timeout=300s \
  deployment/baremetal-operator-controller-manager -n baremetal-operator-system
```

### Install Ironic (Provisioning Service)

```bash
# Deploy Ironic components
kubectl apply -f https://github.com/metal3-io/baremetal-operator/releases/download/${METAL3_VERSION}/ironic.yaml

# Verify
kubectl get pods -n baremetal-operator-system
# Should see: ironic, ironic-inspector, mariadb pods
```

### Verify Installation

```bash
# Check CRDs
kubectl get crds | grep metal3
# Should see: baremetalhosts.metal3.io

# Check operator
kubectl get pods -n baremetal-operator-system

# Check for available hosts
kubectl get baremetalhosts -A
```

---

## Installation

### Install Aether

```bash
# Clone repository
git clone https://github.com/ssahani/aether
cd aether

# Build release binary
cargo build --release

# Install (optional)
sudo cp target/release/aether /usr/local/bin/
```

### Configure kubectl

Ensure kubectl is configured to access your cluster:

```bash
# Test connection
kubectl get nodes

# Set namespace (optional)
export AETHER_NAMESPACE=metal3-system
```

---

## Quick Start

### 1. Prepare Bare Metal Image

Metal3 requires bootable disk images (ISO, QCOW2, or raw):

```bash
# Option 1: Use pre-built image (CoreOS, Ubuntu Server)
IMAGE_URL="https://builds.coreos.fedoraproject.org/prod/streams/stable/builds/38.20231027.3.1/x86_64/fedora-coreos-38.20231027.3.1-metal.x86_64.raw.xz"

# Option 2: Build custom image
# (Create using Packer, Diskimage-builder, etc.)
```

### 2. Create Workload Spec

Create `bare-metal-workload.yaml`:

```yaml
apiVersion: aether/v1
kind: Workload

metadata:
  name: edge-server
  owner: ops-team
  project: infrastructure
  annotations:
    # Required for production: set the boot MAC address of your target server
    aether.io/boot-mac-address: "52:54:00:12:34:56"
    # Required for production: HTTP URL of the bootable disk image
    aether.io/image-url: "http://image-server.local/coreos-stable.img"
    # Optional: checksum URL for image verification
    aether.io/image-checksum-url: "http://image-server.local/coreos-stable.img.sha256sum"

build:
  registry: image-server.local
  # Image will be served from HTTP server

requirements:
  cpu: "16"
  memory: "64Gi"
  storage: "500Gi"

runtime:
  preferred: metal
  allow: [metal]

network:
  service: true
  ports:
    - name: ssh
      port: 22
      protocol: TCP

persistence:
  enabled: true
  size: "500Gi"
  storage_class: "local"
  access_mode: ReadWriteOnce
```

> **Note:** The `aether.io/boot-mac-address` and `aether.io/image-url` annotations are required for production deployments. Without them, Aether uses placeholder values and logs warnings.

### 3. Validate Spec

```bash
aether validate --spec bare-metal-workload.yaml
```

### 4. Provision Server

```bash
# Deploy using Metal3 runtime
aether run --spec bare-metal-workload.yaml --runtime metal

# Output:
# 🚀 Running workload...
# 📦 Selected runtime: metal3
# ✅ Bare metal image reference: edge-server:latest
# ✅ Started instance: edge-server (abc123...)
# Provisioning will begin automatically. Monitor with: kubectl get bmh -n metal3-system
```

### 5. Monitor Provisioning

```bash
# Get host status
aether status edge-server

# Output:
# 📊 Status for 'edge-server':
#   Runtime: metal3
#   State: pending (provisioning)
#   Ready: false

# Watch provisioning progress
kubectl get bmh edge-server -n metal3-system -w
```

### 6. Check Provisioning States

```bash
# Inspect BareMetalHost
kubectl describe bmh edge-server -n metal3-system

# View hardware details
aether logs edge-server
```

---

## Architecture

### How Metal3 Works

```
┌─────────────────────────────────────────┐
│         Aether workload.yaml        │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│       Metal3 Runtime Adapter            │
│  - Parse workload spec                  │
│  - Generate BareMetalHost manifest      │
│  - Add hardware requirements            │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│         Kubernetes API Server           │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│      Baremetal Operator                 │
│  - Watches BareMetalHost CRDs           │
│  - Manages provisioning lifecycle       │
└─────┬───────────────────────┬───────────┘
      │                       │
      ▼                       ▼
┌─────────────┐    ┌──────────────────────┐
│   Ironic    │    │  Ironic Inspector    │
│ (Provision) │    │  (Hardware Discovery)│
└─────┬───────┘    └──────┬───────────────┘
      │                   │
      └───────┬───────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│        BMC (IPMI/Redfish)               │
│  - Power control                        │
│  - Boot order management                │
│  - Serial console access                │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│      Physical Server Hardware           │
│  - CPU, RAM, Disks, NICs, GPUs          │
└─────────────────────────────────────────┘
```

### Provisioning States

```
available → registering → inspecting → preparing → provisioning → provisioned
                                                        ↓
deprovisioning ←────────────────────────────────────────┘
     ↓
 available
```

### Metal3 Annotations

Aether uses workload annotations to configure bare metal provisioning:

| Annotation | Required | Description |
|-----------|----------|-------------|
| `aether.io/boot-mac-address` | **Yes** (production) | Boot MAC address of the target server |
| `aether.io/image-url` | **Yes** (production) | HTTP URL of the bootable disk image |
| `aether.io/image-checksum-url` | No | URL of the image checksum file |

If these annotations are not set, Aether uses placeholder values and logs warnings. This is acceptable for development/testing but must be configured for production.

### Resources Created

For each bare metal deployment, Aether creates:

1. **BareMetalHost** (Metal3 resource)
   - Server specification
   - BMC connection details
   - Image URL and checksum (from annotations or fallback)
   - Hardware requirements (annotations)
   - Online/offline state

2. **Secrets** (for BMC credentials)
   - IPMI/Redfish username
   - IPMI/Redfish password
   - TLS certificates (if needed)

### Generated Manifests

**BareMetalHost Example:**
```yaml
apiVersion: metal3.io/v1alpha1
kind: BareMetalHost
metadata:
  name: edge-server
  namespace: metal3-system
  labels:
    app: edge-server
    managed-by: aether
  annotations:
    aether.io/cpu-cores: "16"
    aether.io/memory-mb: "65536"
    aether.io/gpu-vendor: "nvidia"
    aether.io/gpu-count: "2"
spec:
  online: true
  bootMACAddress: "52:54:00:12:34:56"  # from aether.io/boot-mac-address annotation
  bootMode: UEFI
  bmc:
    address: redfish://192.168.1.100
    credentialsName: edge-server-bmc-secret
  image:
    url: http://image-server/edge-server.img         # from aether.io/image-url annotation
    checksum: http://image-server/edge-server.img.sha256sum  # from aether.io/image-checksum-url
  rootDeviceHints:
    deviceName: /dev/sda
    minSizeGigabytes: 500
  userData:
    name: edge-server-userdata
    namespace: metal3-system
  networkData:
    name: edge-server-networkdata
    namespace: metal3-system
```

---

## Configuration

### CPU Configuration

```yaml
requirements:
  cpu: "32"        # 32 cores
  # OR
  cpu: "16000m"    # 16 cores (16000 millicores)
```

Aether adds CPU requirements as annotations for hardware matching.

### Memory Configuration

```yaml
requirements:
  memory: "128Gi"   # 128 gigabytes
  # OR
  memory: "131072Mi" # 128 GB in mebibytes
```

Converted to MB and stored in annotations.

### Storage Configuration

```yaml
persistence:
  enabled: true
  size: "2000Gi"           # 2TB root disk
  storage_class: "local"    # Local storage
  access_mode: ReadWriteOnce
```

Aether sets `rootDeviceHints.minSizeGigabytes` based on storage requirement.

### Network Configuration

```yaml
network:
  service: true
  ports:
    - name: ssh
      port: 22
      protocol: TCP
    - name: k8s-api
      port: 6443
      protocol: TCP
```

Network configuration determines connectivity requirements.

### GPU Requirements

```yaml
requirements:
  gpu:
    vendor: "nvidia"
    count: 4
```

GPU requirements stored as annotations for matching:
- `aether.io/gpu-vendor: nvidia`
- `aether.io/gpu-count: 4`

---

## Advanced Features

### BMC Configuration

Metal3 supports multiple BMC protocols:

**IPMI:**
```yaml
spec:
  bmc:
    address: ipmi://192.168.1.100
    credentialsName: server-bmc-secret
```

**Redfish:**
```yaml
spec:
  bmc:
    address: redfish://192.168.1.100/redfish/v1/Systems/1
    credentialsName: server-bmc-secret
```

**iDRAC:**
```yaml
spec:
  bmc:
    address: idrac://192.168.1.100/redfish/v1/Systems/System.Embedded.1
    credentialsName: server-bmc-secret
```

### Hardware Discovery

Metal3 automatically discovers hardware during inspection:

```bash
# View discovered hardware
kubectl get bmh edge-server -o yaml | yq '.status.hardware'

# Shows:
# - CPU details (cores, architecture)
# - Memory (total RAM)
# - Storage devices (disks, sizes)
# - NICs (MAC addresses, speeds)
# - Firmware versions
```

### Cloud-Init Integration

Provide user data for server initialization:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: edge-server-userdata
  namespace: metal3-system
stringData:
  userData: |
    #cloud-config
    users:
      - name: admin
        sudo: ALL=(ALL) NOPASSWD:ALL
        ssh_authorized_keys:
          - ssh-rsa AAAAB3...
    packages:
      - docker.io
      - kubernetes-node
    runcmd:
      - systemctl enable docker
      - kubeadm join 10.0.0.1:6443 --token ...
```

### Network Configuration

Custom network setup via networkData:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: edge-server-networkdata
  namespace: metal3-system
stringData:
  networkData: |
    version: 2
    ethernets:
      eno1:
        addresses:
          - 192.168.1.10/24
        gateway4: 192.168.1.1
        nameservers:
          addresses: [8.8.8.8, 8.8.4.4]
      eno2:
        dhcp4: true
```

### Root Device Selection

Specify which disk to use for OS installation:

```yaml
spec:
  rootDeviceHints:
    deviceName: /dev/sda              # Specific device
    # OR
    minSizeGigabytes: 500             # Minimum size
    # OR
    hctl: "0:0:0:0"                   # SCSI address
    # OR
    model: "Samsung SSD 970 EVO"      # Disk model
    # OR
    vendor: "Samsung"                 # Disk vendor
    # OR
    serialNumber: "S4XXNX0M123456"    # Serial number
    # OR
    rotational: false                 # SSD only
```

---

## Operations

### Lifecycle Management

**Provision Server:**
```bash
aether run --spec bare-metal-workload.yaml --runtime metal
```

This creates BareMetalHost with `spec.online: true`, triggering:
1. Registration with Ironic
2. Hardware inspection
3. Image download and checksum verification
4. Disk partitioning and OS installation
5. First boot and cloud-init execution

**Power Off Server:**
```bash
aether stop edge-server
```

Sets `spec.online: false`, powering off the server via BMC.

**Deprovision Server:**
```bash
aether delete edge-server
```

Triggers deprovisioning:
1. Power off server
2. Clean disks (secure erase)
3. Return to available pool
4. Delete BareMetalHost CR

**List Servers:**
```bash
aether list

# Output:
# 📋 Running workloads:
#   edge-server (metal3) - abc123def456
```

### Monitoring

**CLI Status:**
```bash
aether status edge-server

# Shows:
# - Provisioning state
# - Operational status
# - Error messages (if any)
```

**Interactive TUI:**
```bash
aether tui

# Navigate with arrow keys
# Press Enter to view BMC info and hardware details
```

**Direct kubectl:**
```bash
# Get server status
kubectl get bmh edge-server -n metal3-system

# Get detailed info
kubectl describe bmh edge-server -n metal3-system

# Get hardware details
kubectl get bmh edge-server -n metal3-system -o yaml | yq '.status.hardware'
```

### Console Access

**IPMI Serial Console:**
```bash
# Get BMC info
aether logs edge-server

# Connect via IPMI
ipmitool -I lanplus -H 192.168.1.100 -U admin -P password sol activate
```

**Redfish Web Console:**
```bash
# Open BMC web interface
firefox https://192.168.1.100

# Navigate to console/KVM
```

**SSH Access (after provisioning):**
```bash
# Get server IP (from network config or DHCP)
ssh admin@192.168.1.10
```

### Resource Monitoring

**Provisioning Progress:**
```bash
# Watch provisioning
kubectl get bmh edge-server -n metal3-system -w

# Check events
kubectl get events --field-selector involvedObject.name=edge-server -n metal3-system
```

**Ironic Logs:**
```bash
# Check provisioning service logs
kubectl logs -n baremetal-operator-system deployment/ironic

# Check inspector logs
kubectl logs -n baremetal-operator-system deployment/ironic-inspector
```

---

## Troubleshooting

### Common Issues

#### 1. Server Not Registering

**Symptoms:**
```bash
kubectl get bmh edge-server
# STATE: registering (stuck)
```

**Debug:**
```bash
# Check BMC connectivity
ping 192.168.1.100

# Test BMC credentials
ipmitool -I lanplus -H 192.168.1.100 -U admin -P password power status

# Check operator logs
kubectl logs -n baremetal-operator-system \
  deployment/baremetal-operator-controller-manager
```

**Common causes:**
- Incorrect BMC address or credentials
- Network connectivity issues
- BMC firmware compatibility
- Certificate validation failures

**Fix:**
```bash
# Update BMC credentials
kubectl create secret generic edge-server-bmc-secret \
  --from-literal=username=admin \
  --from-literal=password=newpassword \
  -n metal3-system

# Update BareMetalHost
kubectl patch bmh edge-server -n metal3-system --type=merge \
  -p '{"spec":{"bmc":{"credentialsName":"edge-server-bmc-secret"}}}'
```

#### 2. Inspection Failure

**Symptoms:**
```bash
kubectl describe bmh edge-server | grep -A5 Status
# errorMessage: "inspection failed"
```

**Debug:**
```bash
# Check inspector logs
kubectl logs -n baremetal-operator-system deployment/ironic-inspector

# Verify PXE/DHCP
# Verify network boot is enabled in BIOS
```

**Common causes:**
- PXE boot disabled
- DHCP issues
- Network connectivity during boot
- Missing ramdisk images

**Fix:**
```bash
# Enable PXE via BMC
ipmitool -I lanplus -H 192.168.1.100 -U admin -P pass \
  chassis bootdev pxe options=persistent

# Retry inspection
kubectl annotate bmh edge-server -n metal3-system \
  inspect.metal3.io=disabled --overwrite
kubectl annotate bmh edge-server -n metal3-system \
  inspect.metal3.io- --overwrite
```

#### 3. Image Deployment Failure

**Symptoms:**
```bash
# provisioning state stuck at "provisioning"
kubectl get bmh edge-server -o jsonpath='{.status.provisioning.state}'
# Output: provisioning
```

**Debug:**
```bash
# Check image URL accessibility
curl -I http://image-server/edge-server.img

# Verify checksum
curl http://image-server/edge-server.img.sha256sum

# Check Ironic logs
kubectl logs -n baremetal-operator-system deployment/ironic
```

**Common causes:**
- Image URL unreachable from server network
- Checksum mismatch
- Insufficient disk space
- Disk write errors

**Fix:**
```bash
# Verify image is accessible
# Update image URL if needed
kubectl patch bmh edge-server -n metal3-system --type=merge \
  -p '{"spec":{"image":{"url":"http://new-server/edge-server.img"}}}'
```

#### 4. Server Won't Power On

**Symptoms:**
```bash
# online: true but power state: off
kubectl get bmh edge-server -o yaml | grep -A2 power
```

**Debug:**
```bash
# Check power state via BMC
ipmitool -I lanplus -H 192.168.1.100 -U admin -P pass power status

# Try manual power on
ipmitool -I lanplus -H 192.168.1.100 -U admin -P pass power on
```

**Common causes:**
- BMC power control disabled
- Power supply issues
- Server hardware failure
- BMC firmware bug

#### 5. Hardware Mismatch

**Symptoms:**
No available hosts match requirements.

**Debug:**
```bash
# Check available hosts
kubectl get bmh -A

# Check host hardware details
kubectl get bmh -o yaml | yq '.items[].status.hardware'

# Compare with requirements
cat workload-metal.yaml | yq '.requirements'
```

**Fix:**
- Adjust workload requirements
- Provision more servers
- Update hardware annotations

---

## Best Practices

### Resource Planning

**Development/Testing:**
```yaml
requirements:
  cpu: "8"
  memory: "32Gi"
  storage: "500Gi"
```

**Production:**
```yaml
requirements:
  cpu: "32"
  memory: "256Gi"
  storage: "4000Gi"
  gpu:
    vendor: "nvidia"
    count: 4
```

### BMC Security

**Use Secure Credentials:**
```bash
# Generate strong password
BMC_PASSWORD=$(openssl rand -base64 32)

# Store in secret
kubectl create secret generic server-bmc-secret \
  --from-literal=username=admin \
  --from-literal=password="${BMC_PASSWORD}" \
  -n metal3-system
```

**Enable TLS for BMC:**
```yaml
spec:
  bmc:
    address: redfish+https://192.168.1.100/redfish/v1/Systems/1
    disableCertificateVerification: false
```

### Network Isolation

**Separate Networks:**
- **Provisioning Network**: PXE boot, image download
- **BMC Network**: Out-of-band management
- **Production Network**: Application traffic

**Network Policies:**
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: restrict-bmc-access
spec:
  podSelector:
    matchLabels:
      app: baremetal-operator
  policyTypes:
  - Egress
  egress:
  - to:
    - ipBlock:
        cidr: 192.168.1.0/24  # BMC network only
```

### Image Management

**Maintain Image Repository:**
```bash
# Build images
packer build server-image.pkr.hcl

# Generate checksums
sha256sum server-image.raw > server-image.raw.sha256sum

# Serve via HTTP
python3 -m http.server -d /var/lib/images 8080
```

**Version Images:**
```
image-server/
  ├── ubuntu-22.04-v1.0.raw
  ├── ubuntu-22.04-v1.0.raw.sha256sum
  ├── coreos-stable-v38.raw.xz
  └── coreos-stable-v38.raw.xz.sha256sum
```

### High Availability

**Redundant Servers:**
- Provision multiple servers for same workload
- Use load balancer for traffic distribution
- Implement failover mechanisms

**BMC Redundancy:**
- Dual BMC ports where available
- Multiple management network paths

### Monitoring and Alerts

**Prometheus Metrics:**
```bash
# Expose operator metrics
kubectl port-forward -n baremetal-operator-system \
  svc/baremetal-operator-controller-manager-metrics-service 8443:8443
```

**Alert on Provisioning Failures:**
```yaml
alert: BareMetalProvisioningFailed
expr: |
  baremetalhost_provisioning_state{state="error"} == 1
for: 5m
annotations:
  summary: "Server {{ $labels.name }} provisioning failed"
```

---

## Examples

### Example 1: Edge Computing Server

```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: edge-node-01
  owner: edge-team
  project: iot-platform
build:
  registry: image-server.local
requirements:
  cpu: "16"
  memory: "64Gi"
  storage: "1000Gi"
runtime:
  preferred: metal
  allow: [metal]
network:
  service: true
  ports:
    - name: ssh
      port: 22
    - name: mqtt
      port: 1883
persistence:
  enabled: true
  size: "1000Gi"
  storage_class: "local"
  access_mode: ReadWriteOnce
```

### Example 2: GPU Compute Server

```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: ml-server-gpu
  owner: ml-team
  project: ai-training
build:
  registry: image-server.local
requirements:
  cpu: "64"
  memory: "512Gi"
  storage: "8000Gi"
  gpu:
    vendor: "nvidia"
    count: 8
runtime:
  preferred: metal
  allow: [metal]
network:
  service: true
  ports:
    - name: ssh
      port: 22
    - name: jupyter
      port: 8888
persistence:
  enabled: true
  size: "8000Gi"
  storage_class: "nvme"
  access_mode: ReadWriteOnce
```

### Example 3: Database Server

```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: postgres-primary
  owner: database-team
  project: production-db
build:
  registry: image-server.local
requirements:
  cpu: "32"
  memory: "256Gi"
  storage: "4000Gi"
runtime:
  preferred: metal
  allow: [metal]
network:
  service: true
  ports:
    - name: postgres
      port: 5432
persistence:
  enabled: true
  size: "4000Gi"
  storage_class: "ssd"
  access_mode: ReadWriteOnce
```

---

## Comparison: VMs vs Bare Metal

| Feature | KubeVirt (VMs) | Metal3 (Bare Metal) |
|---------|----------------|---------------------|
| **Performance** | Near-native | Full native |
| **Overhead** | Virtualization layer | None |
| **Boot Time** | Minutes | Minutes (similar) |
| **Hardware Access** | Limited (passthrough) | Direct access |
| **Density** | Multiple VMs per host | One workload per server |
| **Resource Isolation** | Strong (virtualization) | Hardware isolation |
| **GPU Support** | Passthrough | Direct access |
| **Use Case** | Multi-tenancy | Maximum performance |

**Use Metal3 when:**
- Maximum performance required
- Direct hardware access needed
- Running GPU-intensive workloads
- Single-tenant servers
- Edge computing scenarios

**Use KubeVirt when:**
- Multi-tenancy needed
- Resource sharing desired
- Density is priority
- Live migration required

---

## Migration Path

### From Traditional Provisioning to Metal3

**Step 1: Inventory Existing Servers**
```bash
# Document BMC addresses
# Document hardware specs
# Create server inventory spreadsheet
```

**Step 2: Prepare Metal3 Environment**
```bash
# Install Metal3 components
# Configure BMC network
# Set up image repository
```

**Step 3: Create BareMetalHost Resources**
```bash
# For each server
cat > server-01.yaml <<EOF
apiVersion: metal3.io/v1alpha1
kind: BareMetalHost
metadata:
  name: server-01
spec:
  bmc:
    address: ipmi://192.168.1.101
    credentialsName: server-01-bmc
  bootMACAddress: "52:54:00:01:01:01"
EOF

kubectl apply -f server-01.yaml
```

**Step 4: Wait for Discovery**
```bash
# Servers will register and be inspected
kubectl get bmh
# All servers should reach "available" state
```

**Step 5: Deploy Workloads via Aether**
```bash
aether run --spec workload-metal.yaml --runtime metal
```

---

## Summary

**Aether + Metal3** provides:

✅ **Easy Bare Metal Provisioning**: One command to provision servers
✅ **Kubernetes Integration**: Servers as native K8s resources
✅ **Unified Management**: Same tool for containers, VMs, and bare metal
✅ **Production Ready**: Full lifecycle management
✅ **Hardware Control**: Direct access to all server hardware

**Next Steps:**

- Try the [Quick Start](#quick-start)
- Provision your first server
- Explore [Advanced Features](#advanced-features)
- Monitor with [TUI dashboard](TUI.md)

**Need Help?**

- Check [Troubleshooting](#troubleshooting)
- Review [Best Practices](#best-practices)
- See [Examples](#examples)

---

**🖧 Start provisioning bare metal with Aether + Metal3 today!**
