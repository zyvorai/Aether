# 🖥️ KubeVirt Deployment Guide

Complete guide for deploying Virtual Machines using Aether's KubeVirt runtime.

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

### What is KubeVirt?

**KubeVirt** is a Kubernetes extension that enables running traditional Virtual Machines alongside containers. It provides:

- **VM Management**: Run VMs as Kubernetes resources
- **Unified Platform**: VMs and containers in one cluster
- **Cloud-Native**: Native Kubernetes APIs and tooling
- **Hardware Access**: Direct GPU and device passthrough

### Why Use Aether with KubeVirt?

Aether simplifies KubeVirt VM deployment:

✅ **One Spec Format**: Same YAML for containers and VMs
✅ **Auto-Generation**: Creates VirtualMachine and DataVolume CRDs
✅ **GPU Support**: Automatic GPU passthrough or vGPU slice configuration
✅ **Live Migration**: `aether live-migrate` moves running VMs between nodes
✅ **Lifecycle Management**: Start, stop, delete VMs easily
✅ **Monitoring**: Integrated TUI dashboard

---

## Prerequisites

### Required Components

1. **Kubernetes Cluster** (v1.24+)
   ```bash
   kubectl version --short
   ```

2. **KubeVirt Installed** (v1.0+)
   ```bash
   kubectl get kubevirt -n kubevirt
   ```

3. **CDI (Containerized Data Importer)**
   ```bash
   kubectl get cdi -n cdi
   ```

4. **virtctl CLI** (optional, for console access)
   ```bash
   curl -Lo virtctl https://github.com/kubevirt/kubevirt/releases/latest/download/virtctl-linux-amd64
   chmod +x virtctl
   sudo mv virtctl /usr/local/bin/
   ```

### Install KubeVirt (if not present)

```bash
# Set version
export KUBEVIRT_VERSION=v1.1.0

# Deploy KubeVirt operator
kubectl apply -f https://github.com/kubevirt/kubevirt/releases/download/${KUBEVIRT_VERSION}/kubevirt-operator.yaml

# Create KubeVirt CR
kubectl apply -f https://github.com/kubevirt/kubevirt/releases/download/${KUBEVIRT_VERSION}/kubevirt-cr.yaml

# Wait for deployment
kubectl wait kv kubevirt --for condition=Available -n kubevirt --timeout=300s
```

### Install CDI

```bash
# Set version
export CDI_VERSION=v1.58.0

# Deploy CDI
kubectl apply -f https://github.com/kubevirt/containerized-data-importer/releases/download/${CDI_VERSION}/cdi-operator.yaml
kubectl apply -f https://github.com/kubevirt/containerized-data-importer/releases/download/${CDI_VERSION}/cdi-cr.yaml

# Wait for deployment
kubectl wait cdi cdi --for condition=Available --timeout=300s
```

### Verify Installation

```bash
# Check KubeVirt
kubectl get pods -n kubevirt

# Check CDI
kubectl get pods -n cdi

# Check CRDs
kubectl get crds | grep kubevirt
# Should see: virtualmachines.kubevirt.io, datavolumes.cdi.kubevirt.io
```

---

## Installation

### Install Aether

```bash
# Clone repository
git clone https://github.com/zyvorai/Aether
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
export AETHER_NAMESPACE=default
```

---

## Quick Start

### 1. Create Workload Spec

Create `vm-workload.yaml`:

```yaml
apiVersion: aether/v1
kind: Workload

metadata:
  name: ubuntu-vm
  owner: ops-team
  project: infrastructure

build:
  registry: docker.io/library
  # Note: Image should be a container disk image
  # Example: docker.io/kubevirt/fedora-cloud-container-disk-demo

requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "20Gi"

runtime:
  preferred: kubevirt
  allow: [kubevirt]

network:
  service: true
  ports:
    - name: ssh
      port: 22
      protocol: TCP

persistence:
  enabled: true
  size: "20Gi"
  storage_class: "local-path"
  access_mode: ReadWriteOnce
```

### 2. Validate Spec

```bash
aether validate --spec vm-workload.yaml
```

### 3. Deploy VM

```bash
# Deploy using KubeVirt runtime
aether run --spec vm-workload.yaml --runtime kubevirt

# Output:
# 🚀 Running workload...
# 📦 Selected runtime: kubevirt
# ✅ VM image reference: ubuntu-vm:latest
# ✅ Started instance: ubuntu-vm (abc123...)
```

### 4. Check Status

```bash
# Get VM status
aether status ubuntu-vm

# Output:
# 📊 Status for 'ubuntu-vm':
#   Runtime: kubevirt
#   State: running
#   Ready: true
```

### 5. Access VM Console

```bash
# View console access instructions
aether logs ubuntu-vm

# Output shows virtctl commands:
# Serial Console:  virtctl console ubuntu-vm
# VNC Access:      virtctl vnc ubuntu-vm
```

### 6. Access VM via virtctl

```bash
# Serial console (text mode)
virtctl console ubuntu-vm

# VNC (graphical mode)
virtctl vnc ubuntu-vm

# SSH (if configured)
kubectl get vmi ubuntu-vm -o jsonpath='{.status.interfaces[0].ipAddress}'
ssh user@<VM-IP>
```

---

## Architecture

### How KubeVirt Works

```
┌─────────────────────────────────────────┐
│         Aether workload.yaml        │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│       KubeVirt Runtime Adapter          │
│  - Parse workload spec                  │
│  - Generate DataVolume manifest         │
│  - Generate VirtualMachine manifest     │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│         Kubernetes API Server           │
└─────┬───────────────────────┬───────────┘
      │                       │
      ▼                       ▼
┌─────────────┐    ┌──────────────────────┐
│ DataVolume  │    │  VirtualMachine      │
│ (CDI)       │    │  (KubeVirt)          │
└─────┬───────┘    └──────┬───────────────┘
      │                   │
      │                   ▼
      │            ┌──────────────────────┐
      │            │  VirtualMachineInstance│
      │            │  (VMI)                │
      │            └──────┬───────────────┘
      │                   │
      ▼                   ▼
┌─────────────────────────────────────────┐
│          Container Runtime (virt-launcher)│
│                  QEMU/KVM                │
└─────────────────────────────────────────┘
```

### Resources Created

For each VM deployment, Aether creates:

1. **DataVolume** (CDI resource)
   - Imports container image to PVC
   - Converts to bootable disk
   - Manages storage lifecycle
   - Aether polls for DataVolume registration (up to 60s) before proceeding with VM creation

2. **VirtualMachine** (KubeVirt resource)
   - Defines VM configuration
   - CPU, memory, devices
   - Running state management

3. **VirtualMachineInstance** (auto-created)
   - Actual running VM
   - Managed by KubeVirt controller

4. **Pod** (auto-created)
   - virt-launcher pod
   - Runs QEMU process

### Generated Manifests

**DataVolume Example:**
```yaml
apiVersion: cdi.kubevirt.io/v1beta1
kind: DataVolume
metadata:
  name: ubuntu-vm-disk
  namespace: default
spec:
  source:
    registry:
      url: docker://docker.io/library/ubuntu-vm:latest
  storage:
    accessModes: [ReadWriteOnce]
    resources:
      requests:
        storage: 20Gi
    storageClassName: local-path
```

**VirtualMachine Example:**
```yaml
apiVersion: kubevirt.io/v1
kind: VirtualMachine
metadata:
  name: ubuntu-vm
  namespace: default
spec:
  running: true
  template:
    spec:
      domain:
        cpu:
          cores: 2
        resources:
          requests:
            memory: 4Gi
        devices:
          disks:
            - name: rootdisk
              disk:
                bus: virtio
          interfaces:
            - name: default
              masquerade: {}
      networks:
        - name: default
          pod: {}
      volumes:
        - name: rootdisk
          dataVolume:
            name: ubuntu-vm-disk
```

---

## Configuration

### CPU Configuration

```yaml
requirements:
  cpu: "4"        # 4 cores
  # OR
  cpu: "2000m"    # 2 cores (2000 millicores)
```

CPU conversion:
- String number → cores directly
- Millicores (`m` suffix) → divided by 1000

### Memory Configuration

```yaml
requirements:
  memory: "8Gi"   # 8 gigabytes
  # OR
  memory: "8192Mi" # 8192 mebibytes
```

### Storage Configuration

```yaml
persistence:
  enabled: true
  size: "50Gi"
  storage_class: "local-path"  # or "ceph-rbd", "nfs", etc.
  access_mode: ReadWriteOnce
```

Supported access modes:
- `ReadWriteOnce`: Single node read-write
- `ReadOnlyMany`: Multiple nodes read-only
- `ReadWriteMany`: Multiple nodes read-write (requires special storage)

### Network Configuration

```yaml
network:
  service: true   # Enable networking
  ports:
    - name: ssh
      port: 22
      protocol: TCP
    - name: http
      port: 80
      protocol: TCP
  type: LoadBalancer  # or ClusterIP, NodePort
```

When `service: true`, VM gets:
- Default pod network interface
- Masquerade mode (NAT)
- External connectivity

When `service: false`:
- No network interfaces
- Isolated VM

### GPU Passthrough

```yaml
requirements:
  gpu:
    vendor: "nvidia"
    model: "Tesla T4"
    count: 2
```

Generated GPU configuration:
```yaml
devices:
  gpus:
    - name: gpu0
      deviceName: nvidia.com/gpu
    - name: gpu1
      deviceName: nvidia.com/gpu
```

**Prerequisites for GPU:**
- NVIDIA GPU operator installed
- KubeVirt GPU feature gate enabled
- Nodes with GPU devices

### vGPU (Mediated Devices)

Set `vgpuProfile` to attach mediated vGPU slices instead of passthrough devices:

```yaml
requirements:
  gpu:
    vendor: "nvidia"
    count: 1
    vgpuProfile: "nvidia.com/GRID_A100-10C"
```

Generated configuration uses the profile as the device name:
```yaml
devices:
  gpus:
    - name: gpu0
      deviceName: nvidia.com/GRID_A100-10C
```

**Prerequisites for vGPU:**
- NVIDIA vGPU Manager installed on the hosts
- The profile listed under KubeVirt `permittedHostDevices.mediatedDevices`
- NVIDIA vGPU software licensing

vGPU is the only GPU mode compatible with live migration — VFIO passthrough
pins the VM to its host, and `aether validate` rejects
`kubevirt.liveMigration` combined with passthrough GPUs.

---

## Advanced Features

### Container Disk Images

KubeVirt uses **container disk** format - a container image containing a VM disk:

**Option 1: Use Pre-built Images**
```yaml
build:
  registry: docker.io/kubevirt
  # Available images:
  # - fedora-cloud-container-disk-demo
  # - cirros-container-disk-demo
  # - alpine-container-disk-demo
```

**Option 2: Build Custom Container Disk**
```dockerfile
FROM scratch
ADD --chown=107:107 ubuntu-20.04.qcow2 /disk/
```

```bash
# Build container disk
podman build -t myregistry/ubuntu-vm:latest .
podman push myregistry/ubuntu-vm:latest
```

### Cloud-Init Configuration

While Aether doesn't directly support cloud-init in the workload spec, you can add it via annotations:

```yaml
metadata:
  annotations:
    kubevirt.io/cloud-init-userdata: |
      #cloud-config
      users:
        - name: ubuntu
          sudo: ALL=(ALL) NOPASSWD:ALL
          ssh_authorized_keys:
            - ssh-rsa AAAAB3...
      packages:
        - nginx
      runcmd:
        - systemctl enable nginx
```

### Live Migration

Aether natively supports KubeVirt live migration. Opt in via the spec:

```yaml
kubevirt:
  liveMigration: true
```

This sets `evictionStrategy: LiveMigrate` on the VMI template (so node drains
migrate the VM instead of shutting it down) and always renders a masquerade
pod-network interface — KubeVirt refuses to migrate VMIs using the default
bridge binding.

Trigger and watch a migration from the CLI:

```bash
# Create a VirtualMachineInstanceMigration and watch its phase
aether live-migrate ubuntu-vm

# Fire-and-forget
aether live-migrate ubuntu-vm --watch-timeout 0
```

**Prerequisites for live migration:**
- A multi-node cluster (single-node migrations stay in `Scheduling` — there is no target)
- Shared/RWX storage for the VM disk is strongly recommended (e.g. CephFS)
- GPU workloads must use `vgpuProfile` (passthrough GPUs cannot migrate; validation rejects the combination)

The equivalent kubectl flow still works:

```bash
kubectl virt migrate ubuntu-vm
kubectl get vmim
```

### Snapshots

Create VM snapshots for backup/restore:

```bash
# Create snapshot
kubectl virt snapshot ubuntu-vm my-snapshot

# Restore from snapshot
kubectl virt restore my-snapshot ubuntu-vm-restored
```

---

## Operations

### Lifecycle Management

**Start VM:**
```bash
aether run --spec vm-workload.yaml --runtime kubevirt
```

**Stop VM:**
```bash
aether stop ubuntu-vm
```

This sets `spec.running: false`, gracefully shutting down the VM.

**Delete VM:**
```bash
aether delete ubuntu-vm
```

Deletes both VirtualMachine and DataVolume resources.

**List VMs:**
```bash
aether list

# Output:
# 📋 Running workloads:
#   ubuntu-vm (kubevirt) - abc123def456
```

### Monitoring

**CLI Status:**
```bash
aether status ubuntu-vm
```

**Interactive TUI:**
```bash
aether tui

# Navigate with arrow keys
# Press Enter to view VM console access info
```

**Direct kubectl:**
```bash
# Get VM status
kubectl get vm ubuntu-vm

# Get VMI (running instance)
kubectl get vmi ubuntu-vm

# Get detailed info
kubectl describe vm ubuntu-vm
```

### Console Access

**Serial Console (text):**
```bash
virtctl console ubuntu-vm
# Press Ctrl+] to exit
```

**VNC (graphical):**
```bash
virtctl vnc ubuntu-vm
# Opens VNC viewer
```

**SSH Access:**
```bash
# Get VM IP
VM_IP=$(kubectl get vmi ubuntu-vm -o jsonpath='{.status.interfaces[0].ipAddress}')

# SSH to VM
ssh user@$VM_IP
```

### Resource Monitoring

**VM Resource Usage:**
```bash
# CPU and memory
kubectl top vmi ubuntu-vm

# Events
kubectl get events --field-selector involvedObject.name=ubuntu-vm
```

---

## Troubleshooting

### Common Issues

#### 1. VM Not Starting

**Symptoms:**
```bash
aether status ubuntu-vm
# State: pending
# Ready: false
```

**Debug:**
```bash
# Check VM events
kubectl describe vm ubuntu-vm

# Check VMI
kubectl get vmi ubuntu-vm

# Check virt-launcher pod
kubectl get pods -l kubevirt.io/vm=ubuntu-vm
kubectl logs -l kubevirt.io/vm=ubuntu-vm
```

**Common causes:**
- DataVolume still importing
- Insufficient resources (CPU/memory)
- Storage class not available
- Image pull errors

#### 2. DataVolume Import Failure

**Symptoms:**
```bash
kubectl get dv ubuntu-vm-disk
# PHASE: ImportInProgress (stuck)
```

**Debug:**
```bash
# Check DataVolume
kubectl describe dv ubuntu-vm-disk

# Check importer pod
kubectl get pods | grep importer
kubectl logs <importer-pod>
```

**Common causes:**
- Invalid image URL
- Registry authentication required
- Network connectivity issues
- Insufficient storage

**Fix:**
```bash
# Delete and recreate
aether delete ubuntu-vm
aether run --spec vm-workload.yaml --runtime kubevirt
```

#### 3. GPU Passthrough Not Working

**Check GPU availability:**
```bash
kubectl describe nodes | grep -A5 "nvidia.com"
```

**Check KubeVirt GPU support:**
```bash
kubectl get kubevirt kubevirt -n kubevirt -o yaml | grep gpu
```

**Enable GPU feature gate:**
```bash
kubectl patch kubevirt kubevirt -n kubevirt --type=merge -p '{"spec":{"configuration":{"developerConfiguration":{"featureGates":["GPU"]}}}}'
```

#### 4. Network Connectivity Issues

**Check VMI network:**
```bash
kubectl get vmi ubuntu-vm -o jsonpath='{.status.interfaces}'
```

**Check pod network:**
```bash
POD=$(kubectl get pods -l kubevirt.io/vm=ubuntu-vm -o name)
kubectl exec $POD -- ip addr
```

#### 5. Console Access Fails

**Check virtctl version:**
```bash
virtctl version
```

**Check VMI is running:**
```bash
kubectl get vmi ubuntu-vm
```

**Alternative access:**
```bash
# Port-forward to SSH
kubectl port-forward vmi/ubuntu-vm 2222:22
ssh -p 2222 user@localhost
```

### Performance Issues

**Slow VM Performance:**

1. **Check resource limits:**
   ```bash
   kubectl get vm ubuntu-vm -o yaml | grep -A5 resources
   ```

2. **Increase CPU/memory:**
   ```yaml
   requirements:
     cpu: "4"      # More cores
     memory: "8Gi" # More RAM
   ```

3. **Use virtio devices:**
   - Aether automatically uses `bus: virtio` for disks
   - Provides best performance

4. **Check storage class:**
   - SSD/NVMe better than HDD
   - Local storage faster than network storage

---

## Best Practices

### Resource Sizing

**Development:**
```yaml
requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "20Gi"
```

**Production:**
```yaml
requirements:
  cpu: "8"
  memory: "32Gi"
  storage: "100Gi"
```

### Storage Classes

**Local development:**
```yaml
persistence:
  storage_class: "local-path"
```

**Production:**
```yaml
persistence:
  storage_class: "ceph-rbd"  # or other HA storage
```

### High Availability

For HA VMs:

1. **Use replicated storage** (Ceph, NFS with replication)
2. **Enable live migration**
3. **Use anti-affinity** (via annotations)
4. **Monitor VM health**

### Security

**Network Policies:**
```bash
# Limit VM network access
kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: vm-network-policy
spec:
  podSelector:
    matchLabels:
      kubevirt.io/vm: ubuntu-vm
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: trusted
    ports:
    - protocol: TCP
      port: 22
EOF
```

**Resource Quotas:**
```bash
# Limit VM resources per namespace
kubectl create quota vm-quota \
  --hard=requests.cpu=16,requests.memory=64Gi \
  -n default
```

### Backup and Disaster Recovery

**Regular snapshots:**
```bash
# Automated snapshot script
kubectl virt snapshot ubuntu-vm ubuntu-vm-$(date +%Y%m%d-%H%M%S)
```

**Export VM disk:**
```bash
# Export to external storage
virtctl image-upload dv ubuntu-vm-disk \
  --image-path=/mnt/backup/ubuntu-vm.img \
  --insecure
```

---

## Examples

### Example 1: Simple Ubuntu VM

```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: ubuntu-basic
  owner: devops
  project: infrastructure
build:
  registry: docker.io/kubevirt
requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "20Gi"
runtime:
  preferred: kubevirt
  allow: [kubevirt]
network:
  service: true
  ports:
    - name: ssh
      port: 22
      protocol: TCP
persistence:
  enabled: true
  size: "20Gi"
  storage_class: "local-path"
  access_mode: ReadWriteOnce
```

### Example 2: High-Performance GPU VM

```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: ml-gpu-vm
  owner: ml-team
  project: machine-learning
  labels:
    workload: ml-training
    gpu: nvidia-t4
build:
  registry: myregistry.io/ml
requirements:
  cpu: "16"
  memory: "64Gi"
  storage: "500Gi"
  gpu:
    vendor: "nvidia"
    model: "Tesla T4"
    count: 4
runtime:
  preferred: kubevirt
  allow: [kubevirt]
network:
  service: true
  ports:
    - name: jupyter
      port: 8888
      protocol: TCP
  type: LoadBalancer
persistence:
  enabled: true
  size: "500Gi"
  storage_class: "ceph-rbd"
  access_mode: ReadWriteOnce
```

### Example 3: Windows VM

```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: windows-server
  owner: ops
  project: infrastructure
build:
  registry: myregistry.io/windows
requirements:
  cpu: "8"
  memory: "16Gi"
  storage: "100Gi"
runtime:
  preferred: kubevirt
  allow: [kubevirt]
network:
  service: true
  ports:
    - name: rdp
      port: 3389
      protocol: TCP
    - name: winrm
      port: 5985
      protocol: TCP
  type: LoadBalancer
persistence:
  enabled: true
  size: "100Gi"
  storage_class: "local-path"
  access_mode: ReadWriteOnce
```

---

## Comparison: Containers vs VMs

| Feature | Podman (Containers) | KubeVirt (VMs) |
|---------|-------------------|----------------|
| **Boot Time** | Seconds | Minutes |
| **Resource Overhead** | Low | Higher (QEMU/KVM) |
| **Isolation** | Process | Full virtualization |
| **OS Support** | Linux only | Any OS (Windows, BSD) |
| **GPU Access** | Limited | Full passthrough |
| **Live Migration** | No | Yes |
| **Use Case** | Microservices | Legacy apps, Windows |

**Use KubeVirt when:**
- Running Windows or non-Linux OS
- Need full virtualization isolation
- Migrating legacy VMs to Kubernetes
- Require GPU passthrough
- Need live migration

**Use Podman/Kubernetes when:**
- Cloud-native applications
- Fast startup required
- Minimal overhead needed
- Linux workloads

---

## Migration Path

### From VMs to Aether/KubeVirt

**Step 1: Export existing VM disk**
```bash
# From VMware/KVM
qemu-img convert -f vmdk vm.vmdk -O qcow2 vm.qcow2
```

**Step 2: Create container disk**
```dockerfile
FROM scratch
ADD --chown=107:107 vm.qcow2 /disk/
```

```bash
podman build -t myregistry/my-vm:latest .
podman push myregistry/my-vm:latest
```

**Step 3: Create Aether spec**
```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: migrated-vm
build:
  registry: myregistry
requirements:
  cpu: "4"
  memory: "8Gi"
  storage: "100Gi"
runtime:
  preferred: kubevirt
  allow: [kubevirt]
# ... rest of spec
```

**Step 4: Deploy**
```bash
aether run --spec migrated-vm.yaml --runtime kubevirt
```

---

## Summary

**Aether + KubeVirt** provides:

✅ **Easy VM Deployment**: One command to launch VMs
✅ **Kubernetes Integration**: VMs as native K8s resources
✅ **Unified Management**: Same tool for containers and VMs
✅ **Production Ready**: Full lifecycle management
✅ **GPU Support**: Automatic passthrough configuration

**Next Steps:**

- Try the [Quick Start](#quick-start)
- Deploy your first VM
- Explore [Advanced Features](#advanced-features)
- Monitor with [TUI dashboard](TUI.md)

**Need Help?**

- Check [Troubleshooting](#troubleshooting)
- Review [Best Practices](#best-practices)
- See [Examples](#examples)

---

**🚀 Start deploying VMs with Aether + KubeVirt today!**
