# Ragnarok and Aether — why two binaries, how they connect

Aether ships confidential computing APIs and KubeVirt manifest wiring **in-process** (`src/ragnarok/`). **Ragnarok** is a separate product binary with its own VM lifecycle UI and attestation services. Both can run alone or together in a **composite** customer bundle.

---

## What was “not wired”

Before dashboard work, Aether already exposed confidential REST endpoints under `/api/confidential/*` (attestation, trust scores, measured images, migration plans). The **web dashboard did not call them** — no nav item, no workload trust tab, no spec editor fields for `confidential:`.

Backend integration (`RAGNAROK_URL` client, deploy attestation gate, KubeVirt `launchSecurity`) was present; **UX was the missing layer**.

The dashboard now includes:

- **Confidential Computing** page (`/confidential`) — fleet trust, TEE capabilities, sovereign mode, measured images
- **Trust** tab on KubeVirt workload detail — per-workload attestation and trust score
- **Attest-gated secrets** — pending/released/revoked status on Trust tab; auto-release on attestation pass (Phase 2)
- **Visual Editor** — optional `confidential:` block when runtime is KubeVirt

---

## Why Ragnarok is a standalone binary

Per [CLIENT_BUNDLE_POLICY.md](../../CLIENT_BUNDLE_POLICY.md), customer tarballs ship **artifacts only** (no source tree):

| Product | Type | Ships |
|---------|------|--------|
| **Aether** | Native binary A | `aether` + embedded dashboard |
| **Ragnarok** | Native binary A | `ragnarok` + `frontend/dist` |

Reasons for separation:

1. **Different primary jobs** — Aether is the universal control plane (Podman, Kubernetes, KubeVirt, Metal3, migration, intent placement). Ragnarok focuses on **confidential VM operations** on KubeVirt (create wizard, attestation UX, image signing, cluster TEE inventory).
2. **Independent release and sizing** — Security teams can deploy Ragnarok on attestation-heavy nodes without pulling the full Aether surface area; dev clusters can run Aether alone with embedded confidential APIs.
3. **Composite packaging** — Enterprise bundles install both binaries side-by-side; they coordinate via env vars and shared data dirs, not a monolithic fork.

This is intentional product architecture, not an unfinished merge.

---

## Architecture

```mermaid
flowchart TB
  subgraph ux [Operator UX]
    AD[Aether Dashboard]
    RD[Ragnarok UI]
  end

  subgraph control [Control plane]
    AS[aether serve]
    RS[ragnarok serve]
  end

  subgraph tee [Confidential layer]
    EM[Embedded src/ragnarok]
    RM[Ragnarok backend]
  end

  subgraph runtime [Runtime]
    KV[KubeVirt VMs]
  end

  AD --> AS
  RD --> RS
  AS --> EM
  AS -->|RAGNAROK_URL optional| RS
  RS --> RM
  EM --> KV
  RM --> KV
```

---

### Composite (enterprise)

Set on the Aether host:

```bash
export RAGNAROK_URL=https://ragnarok.internal:8443
# Optional: share attestation/image state
export RAGNAROK_DATA_DIR=/var/lib/ragnarok
```

- Aether **delegates** remote attestation verify/status to Ragnarok when `RAGNAROK_URL` is set (`src/ragnarok/client.rs`).
- Deploy path can **gate** confidential workloads on attestation (`attestation_gate_for_workload`).
- Aether dashboard shows **composite** mode on the Confidential page and links operators to the Ragnarok UI for VM-centric flows (create VM wizard, live attestation badges).

Both binaries use the same **`confidential:`** block in Aether workload YAML (`src/spec.rs`).

### Phase 3 — measured images (Aether)

| Step | Command / API |
|------|----------------|
| Sign image on pack host | `aether confidential image sign NAME ./disk.qcow2` |
| Verify digest in catalog | `aether confidential image verify-digest DIGEST` |
| List catalog | `aether confidential image list` or GET `/api/confidential/images` |
| Deploy gate | Strict confidential specs require `imageDigest` present in catalog |

### Phase 4 — tenant isolation (Aether)

Fleet policy via `RAGNAROK_ISOLATION_POLICY` JSON (defaults: vTPM, encrypted PVC, NUMA exclusivity, anti-coresharing).

| Check | Command / API |
|-------|----------------|
| Evaluate spec | `aether --spec workload.yaml confidential isolation check` |
| Per-workload | GET `/api/confidential/isolation/:workload` |
| Encrypted disk class | `RAGNAROK_ENCRYPTED_STORAGE_CLASS` env → DataVolume annotation |

KubeVirt manifests receive pod anti-affinity, scheduler hints, and encrypted-PVC annotations automatically when `confidential.enabled`.

### Phase 5 — GuestKit offline inspection (Aether)

Safe inspection without live in-TEE debug. Results persist and feed attestation **Explain**.

| Mode | When | CLI |
|------|------|-----|
| Pre-launch | Before boot | `aether confidential guestkit inspect VM --mode pre-launch --image ./disk.qcow2` |
| Offline policy | Air-gapped | `--mode offline-policy --policy policy.json` |
| Post-shutdown | After VM stop | `--mode post-shutdown --image ./snapshot.qcow2` |
| Attested repair | After failed attestation | `--mode attested-repair` |

| API / UX |
|----------|
| POST `/api/confidential/guestkit/inspect` |
| GET `/api/confidential/guestkit/:vm_id/history` |
| Trust tab → GuestKit section + repair playbook |
| Attestation explain includes last GuestKit summary |

### Phase 7 — Kata / Confidential Containers (Aether)

Kubernetes confidential pods via Kata RuntimeClass (`kata-clh-snp`, `kata-clh-tdx`, etc.).

| Item | Detail |
|------|--------|
| Spec field | `confidential.kataRuntimeClass` |
| Deploy | Auto `runtimeClassName` on Kubernetes workloads |
| API | GET `/api/confidential/kata/status` |
| Example | `examples/confidential-kata-clh-snp.yaml` |
| Manifests | `deploy/confidential/kata-clh/runtime-classes.yaml` |

### Phase 8 — Sovereign cloud (Aether)

| Feature | Env / API |
|---------|-----------|
| BYOK signing | `RAGNAROK_BYOK_SIGNING_KEY` → image sign key id |
| Offline attestation | `RAGNAROK_OFFLINE_ATTESTATION` + `RAGNAROK_CERT_BUNDLE` |
| Region lock | `RAGNAROK_REGION_LOCK` + `confidential.regionLock` |
| Evaluate | GET `/api/confidential/sovereign/evaluate/:workload` |
| CLI | `aether --spec workload.yaml confidential sovereign-check` |

### Phase 9 — Zero-trust networking (Aether)

Cilium auto-policy + PacketWolf hints for confidential workloads.

| Item | Detail |
|------|--------|
| Auto CiliumNetworkPolicy | Applied when none configured in spec |
| PacketWolf annotations | `packetwolf.zyvor.dev/verify-east-west` on pods |
| SPIFFE binding | `spiffe://ragnarok.zyvor.dev/workload/{name}/digest/{digest}` |
| API | GET `/api/confidential/network/:workload` |
| Trust score | Uses real policy count (NetworkPolicy + Cilium + confidential) |

### Phase 10 — AI confidential intelligence (Aether)

| Copilot tool | Purpose |
|--------------|---------|
| `explain_attestation_failure` | Attestation + GuestKit explain |
| `confidential_migrate_plan` | TEE migration blockers |
| `trust_score_fleet` | Fleet AI analysis |
| `confidential_analyze` | Per-workload risk + recommendations |

API: GET `/api/confidential/intelligence` (fleet), GET `/api/confidential/intelligence/:workload`

### Phase 6 — Encrypted live migration (Aether)

Differentiator: TLS+SEV migration channel, re-attestation before cutover, hyper2kvm hints.

| Step | Detail |
|------|--------|
| Plan | `aether --spec workload.yaml confidential migration plan` |
| Strategy | `confidential-blue-green` (deploy target → re-attest → cutover) |
| Channel URI | `tls+sev://qemu+tcp://migrate/...` |
| API | GET `/api/confidential/migration-plan/:name/:target` |
| Status | GET `/api/confidential/migration/:workload/status` |
| Env | `AETHER_CONFIDENTIAL_MIGRATION_ATTEST_TIMEOUT=120` to poll for re-attestation |
| Env | `AETHER_MIGRATION_TARGET_SNP=1` to simulate target TEE in plan |

Example:

```yaml
runtime:
  preferred: kubevirt
  allow: [kubevirt]

confidential:
  enabled: true
  tee: sev-snp
  attestation:
    required: true
    policy: strict
  isolation:
    vtpm: true
    encryptedState: true
    debugAllowed: false
```

See `examples/confidential-snp.yaml`.

---

## When to use which UI

| Task | Use |
|------|-----|
| Deploy/migrate workload from YAML, multi-runtime placement | Aether dashboard / CLI |
| Fleet trust score, attestation explain, policy on existing Aether workloads | Aether → **Confidential** or workload **Trust** tab |
| Create confidential VM, cluster TEE node map, image sign workflow | Ragnarok UI / `rgn confidential` CLI |
| GitOps + intent scoring + confidential KubeVirt | Aether spec + optional Ragnarok for attestation ops |

---

## Environment reference

| Variable | Set on | Purpose |
|----------|--------|---------|
| `RAGNAROK_URL` | Aether | Remote Ragnarok API base for composite attestation |
| `RAGNAROK_DATA_DIR` | Both | Shared attestation/image catalog state (optional) |
| `RAGNAROK_OFFLINE_ATTESTATION` | Both | Sovereign / air-gapped attestation mode |
| `RAGNAROK_REGION_LOCK` | Both | Sovereign region enforcement |
| `VAULT_ADDR` / `VAULT_TOKEN` | Both | HashiCorp Vault KV v2 fetch for attest-gated secret release |
| `VAULT_SECRET_PATH` | Both | Vault path prefix (default `aether/{secret}` or `ragnarok/{secret}`) |
| `KBS_URL` | Both | CoCo Key Broker Service base URL |
| `AETHER_TEE_SNP` | Aether host | Advertise SEV-SNP capability when `/dev/sev` absent in dev |

---

## Related docs

- [Ecosystem overview](../../ECOSYSTEM.md)
- [Client bundle policy](../../CLIENT_BUNDLE_POLICY.md)
- Ragnarok repo: `docs/CONFIDENTIAL.md` (VM-centric confidential computing guide)
