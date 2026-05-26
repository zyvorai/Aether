# Confidential fabric — cluster prerequisites

Install order for Ragnarok + Aether composite confidential workloads:

## 1. Confidential Containers (Kata) — Helm

```bash
./scripts/install-confidential-kata.sh
```

This installs:

- [Confidential Containers](https://github.com/confidential-containers/charts) (`coco` release in `coco-system`)
- [Zyvor RuntimeClasses](../../charts/zyvor-confidential-kata/) (`kata-clh-snp`, `kata-clh-tdx`, optional GPU)

**Requirements:** Helm 3.8+, kubectl, TEE-capable nodes (SEV-SNP / TDX).

On **k3s** / **RKE2**, distribution is auto-detected. Override with `K8S_DISTRIBUTION=k3s`.

## 2. SPIRE (Helm)

```bash
./scripts/install-confidential-spire.sh
./scripts/enable-confidential-production.sh --spire --mount-agent-socket
```

Or install everything:

```bash
./scripts/install-confidential-fabric.sh --enable-backend
```

See [spire/README.md](./spire/README.md).

## 3. Composite Ragnarok + Aether

```bash
# Deploy Aether, then link hub:
../aether/scripts/deploy-remote.sh <host> <user>
./scripts/link-aether-composite.sh   # or full driver:
./scripts/deploy-composite-fabric-remote.sh <host> <user>
```

## Customer bundles

Linux amd64 tarballs include `scripts/install-confidential-kata.sh`, `charts/zyvor-confidential-kata/`, and this directory.
