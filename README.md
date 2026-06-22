<div align="center">

# Aether

### Universal Runtime Control Plane

[![Helm OCI](https://img.shields.io/badge/helm-oci%3A%2F%2Fghcr.io%2Fhypersdk-blue?logo=helm&logoColor=white)](https://github.com/hypersdk/aether)
[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/hypersdk/aether)
[![Trial](https://img.shields.io/badge/trial-30%20days%20free-green)](https://zyvor.dev/aether)
[![License](https://img.shields.io/badge/license-proprietary-red)](https://zyvor.dev/contact)
[![Platform](https://img.shields.io/badge/Kubernetes-1.28%2B-326CE5?logo=kubernetes)](https://zyvor.dev/docs/aether)
[![Confidential](https://img.shields.io/badge/confidential-SEV--SNP%20%7C%20TDX-purple)](https://zyvor.dev/docs/confidential-fabric)

**[Website](https://zyvor.dev/aether)** · **[Docs](https://zyvor.dev/docs/aether)** · **[Contact Sales](mailto:sales@zyvor.dev)**

</div>

---

## What is Aether?

Aether is a **universal runtime control plane** — write one workload spec, deploy to any of four runtimes: Podman containers, Kubernetes, KubeVirt virtual machines, or Metal3 bare metal. With optional confidential computing on SEV-SNP and TDX.

Stop maintaining four separate deployment pipelines. One spec. Four runtimes. Unlimited migration paths.

---

## At a Glance

| Capability | Detail |
|------------|--------|
| **Runtimes** | 4 — Podman · Kubernetes · KubeVirt · Metal3 |
| **Migration paths** | 16 combinations between runtimes |
| **Strategies** | Immediate · Blue-Green · Rolling · Canary · confidential-blue-green |
| **Dashboard pages** | 19 — workloads, migrations, clusters, intent debugger |
| **CLI commands** | 24 — `run`, `migrate`, `drift`, `score`, `attest`, ... |
| **REST API** | 40+ endpoints · OpenAPI at `/api/openapi.json` |
| **Confidential** | SEV-SNP, TDX, Kata/CoCo, measured images, sovereign mode |

---

## One Spec, Four Runtimes

```yaml
# workload.yaml — deploy this ANYWHERE
apiVersion: aether.zyvor.dev/v1
kind: Workload
metadata:
  name: my-app
spec:
  image: my-registry.io/my-app:v1.2.0
  replicas: 3
  runtime: kubernetes          # podman | kubernetes | kubevirt | metal
  intent:
    latency: low
    cost: balanced
    resilience: high
  resources:
    cpu: "500m"
    memory: "512Mi"
```

```bash
# Deploy to Kubernetes
aether run -f workload.yaml --runtime kubernetes

# Migrate to bare metal — no rewrite, no downtime
aether migrate my-app --to metal --strategy blue-green
```

---

## Confidential Computing

Declare a `confidential:` block — Aether wires the rest:

```yaml
spec:
  runtime: kubevirt
  confidential:
    tee: snp                # snp | tdx | kata-snp | kata-tdx
    measured: true
    sovereign: false
```

- Hardware-rooted trust on AMD EPYC (SEV-SNP) and Intel (TDX)
- Kata RuntimeClasses for container-level isolation on the same nodes
- Confidential blue-green migration with re-attestation before cutover
- Composable with Ragnarok for enterprise attestation hub + TEE-gated secrets

---

## Install in 30 Seconds — No Account Required

> The 30-day trial is baked into the binary at build time. No key, no sign-up, no credit card.

**Prerequisites:** Kubernetes 1.28+, Helm 3.8+

```bash
helm install aether oci://ghcr.io/hypersdk/charts/aether \
  --version 0.1.0 \
  --namespace aether-system \
  --create-namespace
```

```bash
# Wait for startup
kubectl -n aether-system rollout status deployment/aether

# API available at
http://$(kubectl get nodes -o jsonpath='{.items[0].status.addresses[0].address}'):30808
```

### Install CLI

```bash
curl -LO https://releases.zyvor.dev/aether/latest/aether-linux-x86_64
chmod +x aether-linux-x86_64 && sudo mv aether-linux-x86_64 /usr/local/bin/aether
aether status
```

---

## Apply a Licence Key (after trial)

```bash
kubectl create secret generic aether-license \
  --from-literal=license.key="<your-key>" \
  -n aether-system

helm upgrade aether oci://ghcr.io/hypersdk/charts/aether \
  --version 0.1.0 --reuse-values \
  --set license.existingSecret="aether-license" \
  -n aether-system
```

Contact **[sales@zyvor.dev](mailto:sales@zyvor.dev)** for a commercial licence.

---

## Client Presentations

Technical and business decks in [`docs/presentations/`](docs/presentations/):

| Deck | Description |
|------|-------------|
| `01-business-value.pdf` | Executive overview — ROI and use cases |
| `02-technical-architecture.pdf` | Architecture deep-dive |
| `03-migration-strategies.pdf` | 16 migration paths explained |
| `04-security-compliance.pdf` | Confidential computing and compliance |
| + 20 more | Full CLI guide, Kubernetes deployment, CI/CD, cost estimation... |

---

## Suite Context

- **[Veyron](https://github.com/hypersdk/veyron)** — KubeVirt VM management & dashboard
- **[Ragnarok](https://github.com/hypersdk/ragnarok)** — AI agents + confidential attestation hub
- **[Hermes](https://github.com/hypersdk/hermes)** — Application operating layer

---

<div align="center">

**Built by [Zyvor Labs](https://zyvor.dev)**

[Website](https://zyvor.dev) · [Docs](https://zyvor.dev/docs/aether) · [Contact](mailto:sales@zyvor.dev)

*Aether is proprietary software. Source code is not public. Binaries are distributed via OCI Helm charts.*

</div>
