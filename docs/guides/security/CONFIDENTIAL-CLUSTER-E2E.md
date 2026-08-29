# Confidential cluster E2E

End-to-end validation for **composite** (Ragnarok + Aether) confidential fabric on a live Kubernetes/KubeVirt lab with SEV-SNP (or dev simulation).

## Prerequisites

| Requirement | Notes |
|-------------|--------|
| Aether `serve` reachable | `AETHER_API`, optional `AETHER_API_KEY` |
| KubeVirt + SNP nodes (live deploy) | Label nodes `node.kubernetes.io/sev-snp=true` or set `AETHER_TEE_SNP=1` on control plane for lab |
| Ragnarok (optional composite) | `RAGNAROK_API` + `deploy-composite-fabric-remote.sh` in Ragnarok repo |
| Kata/CoCo (optional pods path) | `install-confidential-kata.sh` |

## Scripts

| Script | Purpose |
|--------|---------|
| [`scripts/confidential-fabric-e2e.sh`](../../../scripts/confidential-fabric-e2e.sh) | API smoke: capabilities, fleet, composite banner |
| [`scripts/confidential-cluster-e2e.sh`](../../../scripts/confidential-cluster-e2e.sh) | CLI placement + migration plan + optional live deploy/migrate |
| [`scripts/deploy-composite-fabric-remote.sh`](../../../scripts/deploy-composite-fabric-remote.sh) | Deploy Kata + SPIRE + Aether + link Ragnarok hub (needs sibling `ragnarok/` repo) |
| [`scripts/validate-confidential-examples.sh`](../../../scripts/validate-confidential-examples.sh) | Validate all `examples/confidential-*.yaml` |

Makefile targets: `make confidential-validate`, `make confidential-fabric-e2e`, `make confidential-cluster-e2e`.

### API-only smoke

```bash
export AETHER_API=http://<ephemeral-ip>:30090
export AETHER_API_KEY=your-key
export RAGNAROK_API=http://<ephemeral-ip>:30062   # optional

./scripts/confidential-fabric-e2e.sh
```

### Full cluster E2E (CLI, no deploy)

```bash
REQUIRE_SNP=1 ./scripts/confidential-cluster-e2e.sh
```

### Live deploy + placement API + migration plan

```bash
export AETHER_CONFIDENTIAL_LIVE=1
export AETHER_TEE_SNP=1
export KUBECONFIG=~/.kube/config

./scripts/confidential-cluster-e2e.sh
```

### Live confidential-blue-green migrate

```bash
export AETHER_CONFIDENTIAL_LIVE=1
export AETHER_CONFIDENTIAL_MIGRATE=1

./scripts/confidential-cluster-e2e.sh
```

## Composite install (Ragnarok repo)

On the lab host:

```bash
# From Aether (delegates to sibling ragnarok/):
./scripts/deploy-composite-fabric-remote.sh <host> <user>

# Or from Ragnarok directly:
cd ../ragnarok && ./scripts/deploy-composite-fabric-remote.sh <host> <user>
./scripts/confidential-fabric-e2e.sh
```

Then from Aether repo:

```bash
cd Aether
./scripts/confidential-cluster-e2e.sh
```

## What is validated

1. **TEE inventory** — `GET /api/confidential/capabilities` (`sev_snp` when `REQUIRE_SNP=1`)
2. **Placement** — `aether confidential placement` + `GET /api/confidential/placement/:workload` (live)
3. **Migration** — `confidential migration plan` + `GET /api/confidential/migration-plan/:name/kubevirt`
4. **Ragnarok** — `POST /intelligence/placement/confidential` when `RAGNAROK_API` set
5. **Composite** — `GET /api/v1/confidential/composite/status` on Ragnarok

## Related docs

- [RAGNAROK-AND-AETHER.md](RAGNAROK-AND-AETHER.md)
- Ragnarok: `docs/RAGNAROK-AETHER-FABRIC.md`, `docs/CONFIDENTIAL.md`
