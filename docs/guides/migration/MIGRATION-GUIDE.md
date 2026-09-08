# 🔄 Migration Guide

> Complete guide to migrating workloads between runtimes with zero downtime.

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Migration Strategies](#-migration-strategies)
- [Pre-Migration Checklist](#-pre-migration-checklist)
- [Migration Commands](#-migration-commands)
- [Migration Advice AI](#-migration-advice-ai)
- [Rollback](#-rollback)
- [Troubleshooting](#-troubleshooting)

---

## 🌐 Overview

Aether supports migrating workloads between any pair of its three runtimes.

> **Internals:** For state machine, limits matrix, and trace mode see [Migration Internals](MIGRATION-INTERNALS.md) and [Stateful Portability](STATEFUL-PORTABILITY.md).

| From \ To | Podman | Kubernetes | KubeVirt |
|-----------|--------|------------|----------|
| **Podman** | — | ✅ | ✅ |
| **Kubernetes** | ✅ | — | ✅ |
| **KubeVirt** | ✅ | ✅ | — |

That's **9 runtime pair combinations**, each supporting 3 migration strategies.

---

## 🎯 Migration Strategies

### Immediate

Best for: Development, staging, stateless services.

```
1. Stop source workload
2. Start target workload
3. Validate health
4. Update state (or rollback on failure)
```

```bash
aether migrate my-app kubernetes --strategy immediate
```

### Blue-Green (Default)

Best for: Production, critical services requiring zero downtime.

```
1. Start target workload (green)
2. Validate health with exponential backoff
3. Stop source workload (blue)
4. Update state
```

```bash
aether migrate my-app kubernetes --strategy blue-green
```

### Rolling

Best for: Large-scale deployments, canary-style validation.

```
1. Start target with health checks
2. Gradually shift traffic (10% → 25% → 50% → 100%)
3. Health check at each step
4. Complete migration after full validation
```

```bash
aether migrate my-app kubernetes --strategy rolling
```

---

## ✅ Pre-Migration Checklist

| Step | Command | Purpose |
|------|---------|---------|
| 1. Validate spec | `aether validate` | Ensure spec is valid for target runtime |
| 2. Check status | `aether status my-app` | Confirm source is running |
| 3. Create snapshot | Automatic | Aether creates pre-migration snapshot |
| 4. Get advice | `aether migration-advice my-app kube` | AI-powered strategy recommendation |
| 5. Dry run | `aether --dry-run migrate my-app kube` | Preview without executing |
| 6. Run migration | `aether migrate my-app kube` | Execute migration |
| 7. Verify | `aether status my-app` | Confirm target is running |

---

## 💻 Migration Commands

### Migrate

```bash
# Blue-green migration (default)
aether migrate my-app kubernetes

# Immediate migration
aether migrate my-app podman --strategy immediate

# Rolling migration
aether migrate my-app kubevirt --strategy rolling

# Skip health validation (not recommended)
aether migrate my-app kubevirt --no-validation

# Disable automatic rollback
aether migrate my-app kubernetes --no-rollback
```

### Migration Advice

```bash
# Get AI-powered recommendation
aether migration-advice my-app kubernetes
```

Output includes:
- Recommended strategy with reasoning
- Estimated downtime
- Risk level (Low/Medium/High)
- Timing recommendations
- Canary configuration

### Rollback

```bash
# Rollback to latest snapshot
aether rollback my-app
```

### Diff

```bash
# Compare spec vs stored vs live state
aether diff my-app
```

---

## 🤖 Migration Advice AI

The migration advice engine analyzes:

| Factor | What It Checks | Impact |
|--------|----------------|--------|
| **Resource usage** | CPU, memory, storage requirements | Strategy selection |
| **Network dependencies** | Service exposure, ingress, ports | Downtime risk |
| **Persistence** | PVC, storage class, access modes | Data migration complexity |
| **GPU requirements** | Vendor, count | Runtime compatibility |
| **Health probes** | Liveness, readiness checks | Validation capability |
| **Scaling config** | Replicas, HPA settings | Rolling strategy suitability |

---

## 🔧 Troubleshooting

### Migration Fails at Health Check

```bash
# Check target workload status
aether status my-app

# View logs for errors
aether logs my-app

# Rollback if needed
aether rollback my-app
```

### Same-Runtime Migration Blocked

Aether prevents migrating to the same runtime:

```
Error: Workload 'my-app' is already running on Kubernetes.
Hint: Use `aether stop my-app && aether run` to redeploy on the same runtime.
```

### Rollback After Failed Migration

If migration fails and automatic rollback also fails:

```bash
# Manual rollback from snapshot
aether rollback my-app

# If that fails, check snapshots
ls ~/.aether/snapshots/my-app-*
```

---

## 🔗 Related Documentation

| Document | Description |
|----------|-------------|
| [Migration Checklist](../operations/MIGRATION_CHECKLIST.md) | Detailed pre/post migration steps |
| [Migration Strategies Deck](../../client-presentations/03-migration-strategies.html) | Client-facing presentation |
| [Architecture](../../architecture/ARCHITECTURE.md) | Migration engine internals |
