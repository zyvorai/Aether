<!-- Copyright 2026 ZyvorAI Labs Private Limited -- SPDX-License-Identifier: Apache-2.0 -->
# AWS → Kubernetes migration plan (all services)

A repeatable, evidence-based plan for moving an AWS-hosted platform — applications
**and** managed services — onto a Kubernetes platform (K3s/RKE2/vanilla or KubeVirt),
driven by Aether. This is a **cloud-exit / consolidation** methodology:
discover how each application actually runs, score its portability, map every AWS
dependency to a Kubernetes-native target, migrate in risk-ordered waves, prove the
result, and keep it portable afterward.

> Principle: a migration is complete when the application **passes its business,
> data, performance, and security checks** — not merely when Pods are `Running`.

## The flow

```
connect → discover → group into apps → assess (class A–F, portability score)
        → map AWS deps → plan (per-app + project) → rehearse → cut over
        → validate → hypercare → keep portable
```

Aether commands that drive it:

```bash
aether connection add prod-eks --kind eks --context <arn> --kubeconfig ./eks.yaml
aether connection add target   --kind k3s --context <ctx> --kubeconfig ./target.yaml
aether discover start   --connection prod-eks
aether inventory applications --connection prod-eks
aether dependency graph payments --connection prod-eks
aether assess payments --connection prod-eks --target target
aether plan create payments --source prod-eks --target target --strategy blue-green
aether report prod-eks --format md            # Cloud Exit Assessment + AWS mapping

# Execute the Move (stateless / Class-A, blue-green):
aether move plan  payments --source prod-eks --target target --registry <reg> --namespace payments
aether move start payments --source prod-eks --target target --registry <reg>   # shadow deploy
aether move status   payments
aether move cutover  payments   # then point external DNS/Ingress at the target
aether move rollback payments   # deletes the applied target resources
```

`move` reuses the discovery snapshot: it mirrors images to the target registry,
transforms each manifest (strip `status`/`uid`/`nodeName`/`clusterIP`, retarget the
namespace, ECR→registry, AWS `LoadBalancer`→`ClusterIP`, drop IRSA `role-arn`,
AWS endpoints→in-cluster DNS), applies them to the target (ConfigMaps→Services→
workloads), and records a resumable run for cutover/rollback.

Connections use the **kubeconfig** (EKS/AKS/GKE kubeconfigs already carry cloud auth
via exec-plugins). Discovery is **read-only** and never reads Secret *values* — only
references.

## Workload classes (drives the wave order)

| Class | What | Migration approach |
|---|---|---|
| **A** Stateless / portable | web, APIs, workers | replicate image, convert manifests, blue-green/canary |
| **B** Stateful (K8s-native) | PVC-backed DBs/caches on the cluster | app-consistent copy → incremental sync → quiesce → final delta → cut over |
| **C** Cloud-managed dependency | RDS/S3/ElastiCache/… | see AWS-service mapping below (keep / migrate-managed / replace) |
| **D** Privileged / node-dependent | hostNetwork, GPU, devices | node-capability match; may target KubeVirt |
| **E** Operator-managed | CRD-owned (Kafka, DBs) | install CRDs + operator → restore CRs → migrate data |
| **F** Non-portable | naked Pods, `:latest`, no requests | **remediate first**, then re-class |

## AWS managed services → Kubernetes targets

Aether detects these from discovered endpoints and emits them in `aether report`
(the "AWS managed-service dependencies" table). Each row gets a recommended
Kubernetes-native target and migration method.

| AWS service | Kubernetes target | Method | Difficulty |
|---|---|---|---|
| RDS / Aurora (PostgreSQL, MySQL) | **CloudNativePG / Percona on Ceph** (via DataBridge) | logical replication (Debezium CDC) → cutover; verify row counts | high |
| ElastiCache (Redis/Memcached) | Redis / KeyDB | warm replication; caches rebuildable on cutover | medium |
| S3 | **Ceph RGW / MinIO** (S3-compatible, via Atlas) | bucket sync (`rclone`/`mc mirror`) → final delta | low |
| SQS | NATS JetStream / RabbitMQ | drain queues, dual-write during cutover | medium |
| SNS | NATS / Knative Eventing | recreate topics/subscriptions, dual-publish | medium |
| DynamoDB | ScyllaDB / Cassandra | export/import or dual-write; schema remap | high |
| OpenSearch / ES | OpenSearch operator | snapshot/restore or reindex | medium |
| MSK (Kafka) | **Strimzi Kafka** | MirrorMaker 2 topic replication | high |
| Secrets Manager | **Vault + External Secrets** | recreate secrets in Vault; rotate; never copy plaintext | low |
| KMS | Vault Transit | re-wrap data keys | low |
| ECR | **Harbor** | mirror images (`skopeo`) → rewrite refs | low |
| API Gateway | Ingress / Gateway API | recreate routes as K8s resources | low |
| ELB/ALB/NLB | Service `type=LoadBalancer` / MetalLB + Ingress | recreate; update DNS | low |

For each cloud dependency the customer chooses one of three paths: **keep** it in AWS
(hybrid), **migrate to another managed** service, or **replace** with the on-cluster
target above.

## Migration methodology

1. **Onboard & baseline** — owners, windows, RPO/RTO, criticality; baseline live
   CPU/mem/replicas/traffic/error-rate (don't size from YAML requests alone).
2. **Discover & assess** — build the inventory, dependency graph, portability score,
   blockers, and the AWS mapping. Deliverable: the **Cloud Exit Assessment** report.
3. **Remediate** — pin images to digests, add resource requests, wrap naked Pods,
   externalize secrets, remove cloud-specific env/LoadBalancer annotations.
4. **Images** — mirror every image (by digest) to the target registry; verify pull.
5. **Data** — per store: CSI snapshot/restore (compatible storage), filesystem sync
   (cross-vendor), or **application-native replication** (Postgres logical/CDC, MSK
   MirrorMaker, S3 mirror). Verify counts/checksums; track replication lag.
6. **Shadow deploy** — deploy on the target with **no production traffic**; validate
   readiness, endpoints, DNS, storage, secrets, external reachability.
7. **Rehearse** — at least one dry run; record copy/sync/startup/rollback durations;
   update the predicted RTO.
8. **Cut over** — freeze config, lower DNS TTL, final data delta, verify consistency,
   shift 5% → 25% → 50% → 100% while comparing error-rate/latency; keep the source
   available for rollback.
9. **Rollback (automatic)** — on readiness failure / error-rate / latency / data
   inconsistency: shift traffic back, reconcile writes, restore source as authoritative,
   record evidence, block re-cutover until approved.
10. **Hypercare & keep-portable** — enhanced monitoring, daily review; then continuous
    drift detection and scheduled portability re-validation.

## Strategies

- **Cold** (dev/batch): stop → copy → deploy → test → switch.
- **Blue-green** (default for stateless prod): source stays live; target validated; traffic shifts after validation.
- **Canary** (customer-facing): 1% → 5% → 25% → 50% → 100% with per-stage comparison.
- **Active-passive** (DR / conservative stateful): target stays synchronized until promoted.
- **Rolling / parallel-run**: one service at a time, or mirror traffic while only one side commits.

## Acceptance criteria

- **Platform** — expected objects present; Pods ready; no unexpected restarts; services have endpoints; storage attached; backups + monitoring active.
- **Application** — auth, CRUD, background/scheduled jobs, notifications, external APIs, file up/download all work.
- **Data** — row/object counts + checksums match; replication lag zero/accepted; no unprocessed backlog.
- **Performance** — latency + error-rate within approved thresholds; autoscaling tested.
- **Security** — required traffic allowed, unauthorized denied; secrets available but never exposed; RBAC verified; certs valid; audit records generated.

## Where workloads land (Zyvor stack)

```
HyperCluster   target Kubernetes            Aether     discovery + migration
Atlas/Ceph     persistent + S3 storage      PacketWolf dependency + network policy
DataBridge     cloud DB → CNPG/Percona       Zeus OS    day-2 control plane
Veyron         KubeVirt VMs (Class D)
```

See also: [decision-engine/SCORING](../guides/decision-engine/SCORING.md),
[migration internals](../guides/migration/MIGRATION-INTERNALS.md).
