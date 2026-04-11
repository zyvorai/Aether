# ✈️ Migration Checklist

> **Audience:** SREs, DevOps engineers, platform operators
> **License:** Proprietary HyperSDK

A step-by-step operational checklist for migrating workloads between Orchestr8
runtimes (Podman, Kubernetes, KubeVirt, Metal3).

---

## 📑 Table of Contents

- [Pre-Migration Checks](#-pre-migration-checks)
- [Migration Strategy Selection Guide](#-migration-strategy-selection-guide)
- [Execution Steps](#-execution-steps)
  - [Immediate Strategy](#immediate-strategy)
  - [Blue-Green Strategy](#blue-green-strategy)
  - [Rolling Strategy](#rolling-strategy)
- [Post-Migration Verification](#-post-migration-verification)
- [Rollback Procedures](#-rollback-procedures)
- [Troubleshooting](#-troubleshooting)

---

## ✅ Pre-Migration Checks

Complete **every** item before executing a migration.

### 1. Source workload is running

```bash
orchestr8 status <WORKLOAD>
```

- [ ] State is `running`
- [ ] `ready` is `true`
- [ ] Restart count is stable (not increasing)

### 2. Target runtime is available

```bash
# Verify the target runtime is reachable
orchestr8 compare
```

| Target        | Prerequisite                                  |
|---------------|-----------------------------------------------|
| Podman        | `podman` binary in PATH, daemon accessible    |
| Kubernetes    | `kubectl` configured, cluster reachable       |
| KubeVirt      | KubeVirt operator installed on cluster        |
| Metal3        | BMC credentials configured, hardware enrolled |

### 3. Workload spec is valid

```bash
orchestr8 validate
```

- [ ] Validation passes with no errors
- [ ] Spec file path matches `state.spec_path` (check with `orchestr8 diff <WORKLOAD>`)

### 4. Policy check passes on target

```bash
orchestr8 policy-check
```

- [ ] No policy violations
- [ ] Review any warnings and assess risk

### 5. Create a backup

```bash
orchestr8 backup --name "pre-migration-$(date +%Y%m%d)" \
  --description "Before migrating <WORKLOAD> from <SOURCE> to <TARGET>"
```

- [ ] Backup file created in `~/.orchestr8/backups/`
- [ ] Verify backup contents:

```bash
orchestr8 list-backups
```

### 6. Review AI migration advice

```bash
orchestr8 migration-advice <WORKLOAD> <TARGET>
```

- [ ] Review risk assessment
- [ ] Note the recommended strategy
- [ ] Review the estimated migration time

### 7. Check drift (optional but recommended)

```bash
orchestr8 drift <WORKLOAD>
```

- [ ] No critical drift detected
- [ ] If drift exists, reconcile first:

```bash
orchestr8 drift <WORKLOAD> --reconcile
```

### 8. Notify stakeholders

- [ ] Announce maintenance window (if using `immediate` strategy)
- [ ] Configure webhook notifications:

```bash
orchestr8 webhook add migration-alerts \
  "https://hooks.slack.com/services/..." \
  --severity info
```

---

## ⚖️ Migration Strategy Selection Guide

### Comparison table

| Criterion              | Immediate        | Blue-Green          | Rolling             |
|------------------------|------------------|---------------------|---------------------|
| **Downtime**           | Yes (seconds)    | No                  | No                  |
| **Risk**               | Medium           | Low                 | Lowest              |
| **Speed**              | Fastest          | Medium              | Slowest             |
| **Resource overhead**  | None (sequential)| 2x (parallel)       | 2x (parallel)       |
| **Rollback**           | Automatic        | Automatic           | Automatic + granular|
| **Health validation**  | Single check     | Single check        | Retries + per-step  |
| **Traffic shift**      | Instant          | Instant             | Gradual (25/50/75/100%) |
| **Best for**           | Dev/test, non-critical | Prod stateless | Prod critical       |

### Decision flowchart

```
Is downtime acceptable?
├── Yes → Use IMMEDIATE
└── No
    ├── Is the workload stateless?
    │   ├── Yes → Use BLUE-GREEN
    │   └── No → Use ROLLING
    └── Is this a critical production service?
        ├── Yes → Use ROLLING
        └── No → Use BLUE-GREEN
```

### Strategy timing defaults

| Parameter                    | Immediate | Blue-Green | Rolling        |
|------------------------------|-----------|------------|----------------|
| Shutdown delay               | 5s        | 2s         | 5s             |
| Validation delay             | 10s       | 10s        | 10s            |
| Traffic shift interval       | n/a       | n/a        | 5s per step    |
| Cleanup delay                | n/a       | 2s         | 2s             |
| Max health retries           | 1         | 1          | 3              |
| Health retry base interval   | n/a       | n/a        | 2s (exp backoff)|

---

## 🚀 Execution Steps

### Immediate Strategy

**Use case:** Dev/test environments, non-critical workloads where brief downtime
is acceptable.

```bash
orchestr8 migrate <WORKLOAD> <TARGET> --strategy immediate
```

**What happens step by step:**

```
Step 1:  Load workload state from ~/.orchestr8/state.json
Step 2:  Verify source runtime matches expected
Step 3:  Stop source instance
Step 4:  Wait for graceful shutdown (5 seconds)
Step 5:  Create target runtime client
Step 6:  Build container image for target
Step 7:  Deploy to target runtime
         └── On failure + rollback enabled:
             └── Rebuild and redeploy on source → return failure
Step 8:  Wait for validation delay (10 seconds)
Step 9:  Check target health (ready == true)
         └── On failure + rollback enabled:
             └── Delete target, rebuild on source → return failure
Step 10: Delete source instance
Step 11: Update state store with new runtime + instance
Step 12: Done ✔
```

**Example with full flags:**

```bash
orchestr8 migrate api-service kube \
  --strategy immediate \
  --no-validation    # Skip validation delay for fast dev migrations
```

---

### Blue-Green Strategy

**Use case:** Production stateless services. Zero downtime. The target (green)
runs alongside the source (blue) until validated.

```bash
orchestr8 migrate <WORKLOAD> <TARGET> --strategy blue-green
```

**What happens step by step:**

```
Step 1:  Load workload state
Step 2:  Create source + target runtime clients
Step 3:  Build image for target runtime
Step 4:  Deploy to target (green) — source (blue) remains running
         └── On failure: return, blue still running (no cleanup needed)
Step 5:  Wait for validation delay (10 seconds)
Step 6:  Health check green deployment
         └── On failure: delete green, return — blue still running
Step 7:  Switch traffic from blue to green
Step 8:  Wait for connection draining
Step 9:  Stop blue (source) deployment
Step 10: Wait for cleanup delay (2 seconds)
Step 11: Delete blue instance
Step 12: Update state store
Step 13: Done ✔
```

**Example:**

```bash
orchestr8 migrate web-frontend kubevirt --strategy blue-green
```

---

### Rolling Strategy

**Use case:** Critical production services. Gradual traffic shift with health
validation at each step.

```bash
orchestr8 migrate <WORKLOAD> <TARGET> --strategy rolling
```

**What happens step by step:**

```
Phase 1 — Deploy:
  Step 1: Load workload state
  Step 2: Create runtime clients
  Step 3: Build image for target
  Step 4: Deploy to target

Phase 2 — Validate:
  Step 5: Wait for validation delay (10 seconds)
  Step 6: Health check with exponential-backoff retries
          Attempt 1: check health
          Attempt 2: wait 2s, check health
          Attempt 3: wait 4s, check health
          └── All failed: delete target, return — source still running

Phase 3 — Traffic Shift:
  Step 7:  Shift 25% traffic to target → health check
  Step 8:  Shift 50% traffic to target → health check
  Step 9:  Shift 75% traffic to target → health check
  Step 10: Shift 100% traffic to target → health check
           └── At any step, if health fails:
               delete target, return — source still running

Phase 4 — Cleanup:
  Step 11: Stop source deployment
  Step 12: Wait for graceful shutdown (5 seconds)
  Step 13: Delete source instance
  Step 14: Update state store
  Step 15: Done ✔
```

**Example:**

```bash
orchestr8 migrate payment-service kube --strategy rolling
```

---

## 🔎 Post-Migration Verification

After a successful migration, run through this checklist:

### 1. Verify workload status

```bash
orchestr8 status <WORKLOAD>
```

- [ ] State is `running`
- [ ] `ready` is `true`
- [ ] Runtime shows the target runtime

### 2. Check logs for errors

```bash
orchestr8 logs <WORKLOAD>
```

- [ ] No crash loops
- [ ] No error messages
- [ ] Application is responding normally

### 3. Verify health

```bash
orchestr8 health <WORKLOAD> --summary
```

- [ ] Uptime percentage is acceptable
- [ ] Restart count has not increased

### 4. Run a drift check

```bash
orchestr8 drift <WORKLOAD>
```

- [ ] No drift detected between spec and live state

### 5. Test connectivity

```bash
# Port forward and verify
orchestr8 port-forward <WORKLOAD> 8080:80
# Then curl http://localhost:8080 in another terminal
```

- [ ] Application responds correctly

### 6. Verify SLA compliance

```bash
orchestr8 sla check <WORKLOAD> --uptime 99.9 --latency 100
```

- [ ] SLA targets are met

### 7. Review audit trail

```bash
orchestr8 audit --last 10 --workload <WORKLOAD>
```

- [ ] Migration events are recorded
- [ ] No unexpected errors in the trail

### 8. Update monitoring

- [ ] Webhook notifications confirm successful migration
- [ ] External monitors (Prometheus, Grafana) show healthy metrics
- [ ] Alerting thresholds are appropriate for the new runtime

---

## 🔙 Rollback Procedures

### Automatic rollback

By default, Orchestr8 automatically rolls back on migration failure:

- **Immediate strategy:** Rebuilds and redeploys on the source runtime
- **Blue-Green strategy:** Deletes the green deployment; blue remains running
- **Rolling strategy:** Deletes the target; source continues serving 100% traffic

To disable automatic rollback:

```bash
orchestr8 migrate <WORKLOAD> <TARGET> --strategy immediate --no-rollback
```

### Manual rollback

If the automatic rollback did not trigger, or if you discover issues
post-migration:

#### Option 1: Rollback to snapshot

```bash
orchestr8 rollback <WORKLOAD>
```

This restores the workload to its latest pre-deploy snapshot.

#### Option 2: Reverse migration

```bash
orchestr8 migrate <WORKLOAD> <ORIGINAL_RUNTIME> --strategy blue-green
```

#### Option 3: Restore from backup

```bash
# List available backups
orchestr8 list-backups

# Restore the pre-migration backup
orchestr8 restore ~/.orchestr8/backups/pre-migration-20260411.json

# Then redeploy
orchestr8 run --runtime <ORIGINAL_RUNTIME>
```

### Rollback decision matrix

| Scenario                                  | Action                                  |
|-------------------------------------------|-----------------------------------------|
| Migration failed, auto-rollback succeeded | Investigate root cause, retry           |
| Migration failed, auto-rollback failed    | Manual rollback via snapshot or backup  |
| Migration succeeded, issues found later   | Reverse migration or rollback           |
| Migration succeeded, performance degraded | Review with `orchestr8 profile`, adjust |
| State is corrupted                        | Restore from backup                     |

---

## 🔧 Troubleshooting

### "Source and target runtimes are the same"

```
Error: Source and target runtimes are the same (podman).
       Migration is unnecessary.
Hint: Use `orchestr8 rollback <NAME>` to redeploy on the same runtime.
```

**Cause:** You tried to migrate a workload to the runtime it is already on.
**Fix:** Check the current runtime with `orchestr8 status <NAME>`.

---

### "Workload not found"

```
Error: Workload 'my-app' not found
```

**Cause:** The workload is not tracked in the state store.
**Fix:** Run `orchestr8 list` to see all tracked workloads.

---

### "Source runtime mismatch"

```
Error: Source runtime mismatch: expected podman, found kubernetes
```

**Cause:** The workload was migrated or redeployed outside of Orchestr8.
**Fix:** Check the current state with `orchestr8 status <NAME>` and adjust
your migration command accordingly.

---

### Target deployment fails (image pull error)

**Cause:** The target runtime cannot pull the container image.
**Fix:**

1. Verify the registry is accessible from the target
2. Check image pull credentials
3. Rebuild with `orchestr8 build`

---

### Health check fails during rolling migration

**Cause:** The workload is not ready on the target runtime within the
validation window.
**Fix:**

1. Increase health probe `initialDelaySeconds` in the spec
2. Check resource availability on the target runtime:

   ```bash
   orchestr8 schedule utilization
   ```

3. Review logs on the target:

   ```bash
   orchestr8 logs <WORKLOAD>
   ```

---

### Migration is slow

**Cause:** Large images, slow network, or long validation delays.
**Fix:**

1. Skip validation for dev environments:

   ```bash
   orchestr8 migrate <WORKLOAD> <TARGET> --strategy immediate --no-validation
   ```

2. Check build cache:

   ```bash
   orchestr8 build
   ```

3. Review the migration advice:

   ```bash
   orchestr8 migration-advice <WORKLOAD> <TARGET>
   ```

---

### State corruption after failed migration

**Cause:** The migration process was interrupted (e.g., SIGKILL, power loss).
**Fix:**

1. Restore from the pre-migration backup:

   ```bash
   orchestr8 list-backups
   orchestr8 restore <BACKUP_PATH>
   ```

2. Manually check the target runtime for orphaned resources
3. Clean up with the target runtime's native tools (e.g., `kubectl delete`, `podman rm`)

---

### "Circuit breaker open" after migration

**Cause:** The orchestrator's circuit breaker tripped due to repeated health
check failures.
**Fix:**

```bash
# Check the health history
orchestr8 health <WORKLOAD> --last 20

# Reset the circuit breaker
orchestr8 orchestrate reset-circuit <WORKLOAD>

# Run a manual health check
orchestr8 orchestrate health-check
```

---

## 📋 Quick Reference Card

```bash
# Full migration workflow in 6 commands:

# 1. Pre-flight
orchestr8 status my-app
orchestr8 validate
orchestr8 policy-check

# 2. Backup
orchestr8 backup --name pre-migration

# 3. Migrate
orchestr8 migrate my-app kube --strategy blue-green

# 4. Verify
orchestr8 status my-app
orchestr8 health my-app --summary
orchestr8 drift my-app

# 5. (If needed) Rollback
orchestr8 rollback my-app

# 6. Cleanup old backups (keep last 5)
# Manual: review and delete from ~/.orchestr8/backups/
```

---

## 🔗 Cross-References

| Document                                                                     | Description                       |
|------------------------------------------------------------------------------|-----------------------------------|
| [Tutorial 1: Beginner Deployment](../../tutorials/01-beginner-deployment.md) | First deployment walkthrough      |
| [Tutorial 2: Intermediate Workflows](../../tutorials/02-intermediate-workflows.md) | Compose and migration basics |
| [Tutorial 3: Advanced Features](../../tutorials/03-advanced-features.md)     | Policies, secrets, drift          |
| [CLI Reference](../cli/CLI-Reference.md)                                     | Complete command reference        |

---

> 🏷 **License:** Proprietary HyperSDK
