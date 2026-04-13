# 🔄 Migration Guide

Complete guide for migrating workloads between runtimes using Aether's Migration Engine.

---

## 📋 Table of Contents

- [Overview](#overview)
- [Migration Strategies](#migration-strategies)
- [Quick Start](#quick-start)
- [Migration Paths](#migration-paths)
- [Advanced Features](#advanced-features)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)
- [Examples](#examples)

---

## Overview

### What is the Migration Engine?

The **Migration Engine** enables seamless workload migration between different runtimes while maintaining availability and data integrity.

**Key Features:**
- ✅ **Multiple Strategies**: Immediate, Blue-Green, Rolling
- ✅ **Zero-Downtime**: Blue-Green and Rolling strategies
- ✅ **Automatic Rollback**: On failure detection
- ✅ **State Management**: Preserves workload state across migrations
- ✅ **Health Validation**: Ensures target instance is ready
- ✅ **All Runtime Pairs**: Migrate between any two runtimes

### Supported Migration Paths

```
🐳 Podman ←→ ☸️ Kubernetes ←→ 🖥️ KubeVirt ←→ 🖧 Metal3
   ↓            ↓               ↓              ↓
   └────────────┴───────────────┴──────────────┘
          Any-to-Any Migrations
```

**Bidirectional migrations supported:**
- Podman ↔ Kubernetes
- Podman ↔ KubeVirt
- Podman ↔ Metal3
- Kubernetes ↔ KubeVirt
- Kubernetes ↔ Metal3
- KubeVirt ↔ Metal3

---

## Migration Strategies

### 1. Immediate Migration

**Description:** Stop source, start target immediately.

**Characteristics:**
- Fastest migration
- Brief downtime (seconds to minutes)
- Minimal resource usage
- Simple rollback

**Use Cases:**
- Development/testing environments
- Non-critical workloads
- Scheduled maintenance windows
- Quick runtime switching

**Process:**
```
1. Stop source instance
2. Deploy to target runtime
3. Validate target instance
4. Delete source instance
5. Update state
```

**Downtime:** Yes (brief)

**Example:**
```bash
aether migrate my-app kubernetes --strategy immediate
```

### 2. Blue-Green Migration

**Description:** Deploy to target (green) while source (blue) runs, then switch.

**Characteristics:**
- Zero downtime
- Requires 2x resources temporarily
- Quick rollback (switch back)
- Validation before switch

**Use Cases:**
- Production workloads
- Critical services
- When zero downtime is required
- A/B testing scenarios

**Process:**
```
1. Deploy to target (green) while source (blue) runs
2. Validate green instance
3. Switch traffic from blue to green
4. Monitor for issues
5. Delete blue instance
```

**Downtime:** None

**Example:**
```bash
aether migrate my-app kubernetes --strategy blue-green
```

### 3. Rolling Migration

**Description:** Gradual traffic shift with continuous validation.

**Characteristics:**
- Zero downtime
- Gradual transition (25%, 50%, 75%, 100%)
- Continuous health checks
- Safest for critical workloads

**Use Cases:**
- Mission-critical services
- High-traffic applications
- When gradual validation is needed
- Risk-averse migrations

**Process:**
```
1. Deploy to target runtime
2. Validate target instance
3. Shift 25% traffic → validate
4. Shift 50% traffic → validate
5. Shift 75% traffic → validate
6. Shift 100% traffic → validate
7. Delete source instance
```

**Downtime:** None

**Example:**
```bash
aether migrate my-app kubernetes --strategy rolling
```

---

## Quick Start

### Prerequisites

1. **Source workload running**
   ```bash
   aether list
   # Should show your workload
   ```

2. **Target runtime available**
   ```bash
   # For Kubernetes
   kubectl cluster-info

   # For Podman
   podman version

   # For KubeVirt
   kubectl get kubevirt -n kubevirt

   # For Metal3
   kubectl get bmh -A
   ```

### Basic Migration

**Step 1: Check current state**
```bash
aether status my-app
# Note current runtime
```

**Step 2: Execute migration**
```bash
# Blue-green migration (recommended)
aether migrate my-app kubernetes --strategy blue-green

# Output:
# 🔄 Migrating workload 'my-app'...
# 📊 Migration Plan:
#   Workload: my-app
#   Source: podman
#   Target: kubernetes
#   Strategy: BlueGreen
# ✅ Migration completed successfully!
```

**Step 3: Verify**
```bash
aether status my-app
# Should show new runtime

aether tui
# Visualize runtime change
```

---

## Migration Paths

### Container → Kubernetes

**Use Case:** Moving from local development to production cluster

**Example:**
```bash
# Start on Podman
aether run --spec app.yaml --runtime podman

# Migrate to Kubernetes
aether migrate my-app kubernetes --strategy blue-green
```

**What Happens:**
1. Workload spec converted to Pod/Service/PVC
2. Container deployed to Kubernetes
3. Health probes validated
4. Podman container removed

**Considerations:**
- Kubernetes requires registry access
- Volume mounts converted to PVCs
- Network ports become Services
- Resource limits applied

### Kubernetes → KubeVirt

**Use Case:** Moving to VM for isolation/legacy OS support

**Example:**
```bash
# Start on Kubernetes
aether run --spec app.yaml --runtime kubernetes

# Migrate to KubeVirt
aether migrate my-app kubevirt --strategy rolling
```

**What Happens:**
1. Container image converted to VM disk (DataVolume)
2. VirtualMachine created with same resources
3. Gradual traffic shift
4. Pod/Service deleted

**Considerations:**
- Longer boot time for VMs
- Different console access (virtctl)
- Direct hardware access available
- May need cloud-init configuration

### KubeVirt → Metal3

**Use Case:** Moving VM to bare metal for maximum performance

**Example:**
```bash
# Start on KubeVirt
aether run --spec app.yaml --runtime kubevirt

# Migrate to Metal3
aether migrate my-app metal --strategy immediate
```

**What Happens:**
1. VM disk image prepared for bare metal
2. BareMetalHost provisioned
3. Physical server boots and installs
4. VM deleted

**Considerations:**
- Requires available bare metal hosts
- Provisioning takes longer (hardware boot)
- BMC access needed
- Hardware-specific configuration

### Podman → Metal3

**Use Case:** Moving container directly to dedicated hardware

**Example:**
```bash
# Start on Podman
aether run --spec app.yaml --runtime podman

# Migrate to Metal3
aether migrate my-app metal --strategy blue-green
```

**What Happens:**
1. Container converted to bootable bare metal image
2. Server provisioned with image
3. Validation after provisioning
4. Container removed

**Considerations:**
- Significant architecture change
- Boot time much longer
- Exclusive hardware resource access
- Best for performance-critical workloads

---

## Advanced Features

### Validation Delay

Control how long to wait before considering target healthy:

```bash
# Default: 30 seconds
aether migrate my-app kubernetes --strategy blue-green

# No validation (fast but risky)
aether migrate my-app kubernetes --strategy blue-green --no-validation
```

The migration engine uses configurable timing parameters with sensible defaults. Configure via `MigrationPlan::new()` in the API or `~/.aether/config.yaml`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `validation_delay` | 10s | Wait before checking target health |
| `shutdown_delay` | 5s | Graceful shutdown delay before starting target |
| `traffic_shift_interval` | 5s | Delay between rolling traffic-shift steps |
| `cleanup_delay` | 2s | Delay before cleaning up source |
| `max_health_retries` | 3 | Health check retry attempts |
| `health_retry_base_interval` | 2s | Base interval for exponential backoff |

Health check retries use **exponential backoff** (base * 2^attempt, capped at 30s).

```yaml
migration:
  trafficSwitchDelaySecs: 10
  gracefulShutdownSecs: 5
```

### Same-Runtime Guard

Migrating to the same runtime is rejected with a helpful error:

```bash
$ aether migrate my-app podman
# Error: Source and target runtimes are the same (podman). Migration is unnecessary.
# Hint: Use `aether rollback my-app` to redeploy on the same runtime.
```

### Dry Run

Preview what a migration would do without executing:

```bash
aether migrate my-app kubernetes --strategy blue-green --dry-run
```

### Automatic Rollback

Enable/disable rollback on failure:

```bash
# Rollback enabled (default)
aether migrate my-app kubernetes --strategy immediate

# Rollback disabled
aether migrate my-app kubernetes --strategy immediate --no-rollback
```

**With Rollback:**
- On failure, source instance restored
- State reverted to pre-migration
- User notified of rollback

**Without Rollback:**
- Failure stops migration
- Source instance left in stopped state
- Manual intervention required

### State Preservation

Migration engine automatically:
- Preserves workload metadata
- Updates runtime information
- Maintains creation timestamp
- Updates modification timestamp
- Keeps spec file reference

**State File Before:**
```json
{
  "my-app": {
    "name": "my-app",
    "runtime": "podman",
    "instance": {
      "id": "abc123",
      "name": "my-app-container"
    }
  }
}
```

**State File After:**
```json
{
  "my-app": {
    "name": "my-app",
    "runtime": "kubernetes",
    "instance": {
      "id": "def456",
      "name": "my-app-pod"
    },
    "updated_at": "2024-01-15T10:30:00Z"
  }
}
```

---

## Best Practices

### Choose the Right Strategy

**Immediate:**
- ✅ Development environments
- ✅ Testing workloads
- ✅ Off-peak hours
- ✅ Stateless applications
- ❌ Production critical services
- ❌ High availability requirements

**Blue-Green:**
- ✅ Production workloads
- ✅ Web applications
- ✅ API services
- ✅ When quick rollback needed
- ⚠️ Requires 2x resources temporarily
- ⚠️ Traffic switch must be planned

**Rolling:**
- ✅ Mission-critical services
- ✅ High-traffic applications
- ✅ Gradual risk mitigation
- ✅ Continuous validation needed
- ⚠️ Longest migration time
- ⚠️ Complex rollback

### Pre-Migration Checklist

**1. Verify Source Health**
```bash
aether status my-app
# Ensure: State=running, Ready=true
```

**2. Check Target Runtime**
```bash
# Kubernetes
kubectl get nodes
kubectl get sc  # Storage classes

# KubeVirt
kubectl get kubevirt -n kubevirt

# Metal3
kubectl get bmh -A
```

**3. Backup State**
```bash
cp ~/.aether/state.json ~/.aether/state.json.backup
```

**4. Review Workload Spec**
```bash
aether validate --spec workload.yaml
```

**5. Test in Non-Production First**
```bash
# Clone workload spec
cp workload.yaml workload-test.yaml

# Deploy test instance
aether run --spec workload-test.yaml --runtime podman

# Test migration
aether migrate test-app kubernetes --strategy blue-green

# Verify
aether status test-app

# Cleanup
aether delete test-app
```

### During Migration

**Monitor Progress:**
```bash
# In separate terminal
watch -n 2 'aether list'

# Or use TUI
aether tui
```

**Check Logs:**
```bash
# Run with verbose logging
aether -v migrate my-app kubernetes --strategy blue-green
```

**Prepare for Rollback:**
- Keep source runtime accessible
- Have backup deployment ready
- Monitor application metrics
- Watch for error spikes

### Post-Migration

**1. Verify Instance**
```bash
aether status my-app
# Ensure: State=running, Ready=true
```

**2. Test Functionality**
```bash
# For web apps
curl http://my-app-endpoint

# For APIs
curl http://my-app-api/health
```

**3. Monitor Performance**
- CPU usage
- Memory usage
- Response times
- Error rates

**4. Update Documentation**
- Record migration date
- Update architecture diagrams
- Note any issues encountered
- Document rollback procedure

### Rollback Procedure

If migration fails or issues detected:

**Automatic Rollback:**
```bash
# With --no-rollback NOT set
aether migrate my-app kubernetes --strategy blue-green
# On failure, automatic rollback to source
```

**Manual Rollback:**
```bash
# If already migrated and need to go back
aether migrate my-app podman --strategy immediate

# Verify
aether status my-app
```

**Emergency Rollback:**
```bash
# Restore from backup state
cp ~/.aether/state.json.backup ~/.aether/state.json

# Verify source still running
kubectl get pods my-app  # or
podman ps | grep my-app

# Delete target if created
aether delete my-app --runtime kubernetes
```

---

## Troubleshooting

### Migration Fails Immediately

**Symptoms:**
```
❌ Migration failed!
  Error: Workload 'my-app' not found
```

**Cause:** Workload not in state store

**Fix:**
```bash
# List workloads
aether list

# Check name spelling
# Re-deploy if needed
aether run --spec workload.yaml
```

### Target Instance Not Ready

**Symptoms:**
```
❌ Migration failed!
  Error: Target failed validation after retries
  Rollback: Performed successfully
```

**Cause:** Target instance failing health checks

**Debug:**
```bash
# Check target runtime
kubectl get pods  # for Kubernetes
kubectl get vmi   # for KubeVirt
kubectl get bmh   # for Metal3

# Check logs
kubectl logs my-app  # for Kubernetes
```

**Fix:**
- Verify resource requirements are met
- Check image availability
- Review configuration
- Increase validation delay

### Source Runtime Mismatch

**Symptoms:**
```
Error: Source runtime mismatch: expected podman, found kubernetes
```

**Cause:** State file shows different runtime than expected

**Fix:**
```bash
# Check actual runtime
aether status my-app

# Use correct source runtime in migration plan
# Or specify target only (auto-detects source)
```

### Resource Constraints

**Symptoms:**
Blue-Green migration fails due to insufficient resources

**Debug:**
```bash
# Check cluster capacity
kubectl describe nodes

# Check resource usage
kubectl top nodes
kubectl top pods
```

**Fix:**
- Use Immediate strategy instead
- Scale down other workloads temporarily
- Add cluster capacity
- Schedule during off-peak hours

### Network Issues

**Symptoms:**
Migration succeeds but service unreachable

**Debug:**
```bash
# Check service
kubectl get svc my-app

# Check endpoints
kubectl get endpoints my-app

# Test connectivity
kubectl run -it --rm debug --image=busybox --restart=Never -- wget -O- my-app:8080
```

**Fix:**
- Verify Service creation
- Check network policies
- Review ingress configuration
- Test from within cluster

---

## Examples

### Example 1: Dev to Production

```bash
# Development: Podman
aether run --spec app.yaml --runtime podman

# Test locally
curl http://localhost:5090

# Migrate to Kubernetes (production)
aether migrate my-app kubernetes --strategy blue-green

# Verify in production
kubectl get pods
curl http://my-app.example.com
```

### Example 2: Container to VM

```bash
# Start with container
aether run --spec app.yaml --runtime kubernetes

# Need VM for Windows dependency
aether migrate my-app kubevirt --strategy rolling

# Access VM
virtctl console my-app
```

### Example 3: VM to Bare Metal

```bash
# Start with VM
aether run --spec ml-workload.yaml --runtime kubevirt

# Need GPU passthrough - migrate to bare metal
aether migrate ml-workload metal --strategy immediate

# Monitor provisioning
kubectl get bmh ml-workload -w
```

### Example 4: Emergency Rollback

```bash
# Migrate to new runtime
aether migrate my-app kubernetes --strategy blue-green

# Detect issues
curl http://my-app:8080/health
# Error: 500 Internal Server Error

# Rollback immediately
aether migrate my-app podman --strategy immediate

# Verify rollback successful
aether status my-app
# Runtime: podman, State: running
```

### Example 5: Multi-Stage Migration

```bash
# Stage 1: Local to Kubernetes
aether migrate my-app kubernetes --strategy blue-green
sleep 60  # Monitor

# Stage 2: Kubernetes to KubeVirt (need better isolation)
aether migrate my-app kubevirt --strategy rolling
sleep 120  # Monitor

# Stage 3: KubeVirt to Metal3 (need max performance)
aether migrate my-app metal --strategy immediate
```

### Example 6: Canary Migration

```bash
# Main workload on Podman
aether run --spec app.yaml --runtime podman

# Create test instance for canary
cp workload.yaml workload-canary.yaml
# Edit: Change name to "my-app-canary"

# Deploy canary to Kubernetes
aether run --spec workload-canary.yaml --runtime kubernetes

# Test canary
# ... monitor metrics ...

# If successful, migrate main workload
aether migrate my-app kubernetes --strategy blue-green

# Cleanup canary
aether delete my-app-canary
```

---

## Migration Decision Matrix

| Scenario | Recommended Strategy | Reason |
|----------|---------------------|---------|
| Dev → Prod | Blue-Green | Zero downtime, quick rollback |
| Scheduled Maintenance | Immediate | Fastest, downtime acceptable |
| High Traffic Service | Rolling | Gradual validation, safest |
| Testing Runtime | Immediate | Quick, downtime doesn't matter |
| Mission Critical | Rolling | Maximum safety, validation |
| Resource Constrained | Immediate | No 2x resources needed |
| Quick Experiment | Immediate | Fastest turnaround |
| Production API | Blue-Green | Zero downtime, validated |

---

## Performance Considerations

### Migration Duration

**Immediate:**
- Podman → Kubernetes: 30-60 seconds
- Kubernetes → KubeVirt: 2-5 minutes (VM boot)
- KubeVirt → Metal3: 10-30 minutes (server provisioning)
- Podman → Metal3: 10-30 minutes

**Blue-Green:**
- Add deployment time + configurable traffic switch delay (default: validation_delay from MigrationPlan)
- Example: Podman → Kubernetes = 60-90 seconds

**Rolling:**
- Add deployment time + configurable traffic shift intervals
- Example: Podman → Kubernetes = 90-120 seconds

### Resource Usage

**Immediate:**
- Source resources + minimal overhead
- Peak: 1x during deployment

**Blue-Green:**
- Source + Target running simultaneously
- Peak: 2x resources
- Duration: Validation period (30s default)

**Rolling:**
- Source + Target running simultaneously
- Peak: 2x resources
- Duration: Full migration (60s+)

---

## Summary

The Migration Engine provides:

✅ **Flexible Strategies**: Choose based on requirements
✅ **Zero-Downtime**: Blue-Green and Rolling options
✅ **Automatic Rollback**: Safety net for failures
✅ **All Runtime Pairs**: Any-to-any migration
✅ **State Preservation**: Maintains workload tracking
✅ **Validation**: Health checks before commit

**Best Practices:**
1. Test migrations in non-production first
2. Backup state before migrating
3. Use appropriate strategy for workload criticality
4. Monitor during and after migration
5. Have rollback plan ready

**Next Steps:**
- Try your first migration
- Experiment with different strategies
- Set up monitoring for migrations
- Document your migration procedures

---

**🔄 Seamless runtime switching with Aether Migration Engine!**
