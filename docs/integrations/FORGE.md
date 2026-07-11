<!-- Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved. -->
# Forge GPU / AI integration

[Forge](https://github.com/ssahani/forge) is the Zyvor **AI infrastructure control
plane** on Kubernetes. When enabled, Aether reads GPU capacity, node inventory,
placement recommendations, and cost from the Forge API gateway — so operators can see
and reason about GPU/AI infrastructure from the same CLI and dashboard they use for
everything else. The integration is **read-only**: Aether queries Forge, it does not
mutate Forge state.

## Enabling

Set on the Aether CLI or `aether serve` gateway environment:

| Variable | Meaning |
|----------|---------|
| `AETHER_FORGE_URL` | Forge gateway base URL, e.g. `http://forge-api-gateway.forge.svc.cluster.local:24631`. **Unset ⇒ integration off.** (`FORGE_API_URL` is also accepted.) |
| `AETHER_FORGE_TOKEN` | Optional `Authorization: Bearer` API key. (`FORGE_API_KEY` is also accepted.) |

`ForgeConfig::from_env()` returns "not configured" unless a non-empty URL is set — no
default endpoint is assumed.

## CLI

```bash
aether forge stats                              # cluster GPU stats: total/available/allocated, utilization, jobs
aether forge nodes                              # GPU-capable nodes and their GPU counts
aether forge recommend --gpu-type A100 --gpus 2 [--model llama-70b]
                                                # ranked placement recommendation for a GPU/AI request
aether forge cost                               # GPU cost breakdown
```

When Forge is unset, commands print `Forge is not configured. Set AETHER_FORGE_URL…`.

## REST

Viewer role, served by the Aether gateway:

| Endpoint | Returns |
|----------|---------|
| `GET /api/forge/stats` | Cluster GPU stats, with `configured: true`. Returns `{ "configured": false }` when Forge is unset. |
| `GET /api/forge/nodes` | GPU node list (`items[]` unwrapped). |

These proxy Forge's `GET /api/cluster/stats` and `GET /api/nodes`. Placement
(`/api/ai/placement/recommend`) and cost (`/api/metrics/costs`) are available via the CLI.

## Web dashboard

A read-only **GPU / Forge** page under **Resources** shows GPU capacity tiles
(Total / Available / Allocated / Utilization %) and a GPU-nodes table. When Forge is
not configured it renders a "Forge not configured" empty state instead of erroring.

## Deploy wiring

`scripts/deploy-remote.sh` / `scripts/lib-deploy-manifest.sh` wire Forge into the
in-cluster deployment **opt-in**:

```bash
# Enable with the in-cluster gateway default and a token:
AETHER_FORGE_ENABLE=1 AETHER_FORGE_TOKEN=<key> ./scripts/deploy-remote.sh <host> <user>

# Or point at an explicit gateway:
AETHER_FORGE_URL=http://forge-api-gateway.forge.svc.cluster.local:24631 \
AETHER_FORGE_TOKEN=<key> ./scripts/deploy-remote.sh <host> <user>
```

- `AETHER_FORGE_URL` is injected as a plain env var (defaults to the in-cluster gateway
  when only `AETHER_FORGE_ENABLE=1` is given).
- `AETHER_FORGE_TOKEN`, when set, is stored in a Kubernetes Secret `aether-forge`
  (key `token`) and mounted via `secretKeyRef` — **never** baked into the manifest.

> **Security:** the Forge lab default key `Admin@321` is an insecure placeholder. Rotate
> it before any non-lab use and supply the real key via `AETHER_FORGE_TOKEN`.

## Local end-to-end

```bash
# 1. Run Forge locally (see ../forge); note the gateway port (default 24631).
# 2. Point Aether at it and inspect:
AETHER_FORGE_URL=http://127.0.0.1:24631 AETHER_FORGE_TOKEN=<key> aether forge stats
AETHER_FORGE_URL=http://127.0.0.1:24631 AETHER_FORGE_TOKEN=<key> aether forge nodes
```

A CPU-only cluster correctly reports `0` GPUs — that is a healthy "connected, no GPUs"
result, not an error.
