# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-31 · Resume with **"cont"** to ship the next batch.

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced with `origin/main` after v12 push) |
| Commit | v12 Era G — `feat(ai-os): ship copilot & LLM platform v12 (phases 65–74)` |
| Uncommitted | `web/dashboard/node_modules/*` only (safe to discard: `git checkout -- web/dashboard/node_modules`) |

## Shipped (v1–v12)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v1–v4 | Core panels | 1–4 | Command Center, Copilot, topology, intent pipeline |
| v5–v7 | A–B | 5–24 | Autonomous execute, agent loop, Intent platform |
| v8 | C Multi-Cloud | 25–34 | `federation_os.rs`, federation panels |
| v9 | D SRE | 35–44 | `sre_os.rs`, SRE reliability panel |
| v10 | E Knowledge Graph | 45–54 | `graph_os.rs`, graph platform |
| v11 | F macOS Native | 55–64 | `macos_os.rs`, Tauri shell, `MacOSPlatformPanel` |
| v12 | G Copilot & LLM | 65–74 | `copilot_os.rs`, `CopilotPlatformPanel`, `aether copilot` CLI |

Docs: `docs/PHASES-AI-OS.md`, `docs/ROADMAP.md` (eras marked **Ship** through phase 74).

## Next batch — v13 Era H (phases 75–84)

**FinOps & Cost** — see `docs/PHASES-AI-OS.md` § Era H.

| # | Phase | Focus |
|---|-------|-------|
| 75 | Chargeback automation | Owner/project auto-attribution |
| 76 | Spot/preemptible advisor | Workload → spot eligibility |
| 77 | Reserved instance planner | RI/SP recommendation engine |
| 78 | Cost anomaly detection | Spend spike alerts |
| 79 | Unit economics | Cost per request metric |
| 80 | FinOps agent execute | Auto-downsize on schedule |
| 81 | Multi-cloud cost compare | Live AWS/GCP/Azure in pipeline |
| 82 | Budget guardrails | Hard/soft spend caps |
| 83 | Cost forecast API | ML spend projection |
| 84 | FinOps dashboard panel | Unified cost intelligence UI |

**Implementation pattern (repeat per batch):**

1. `src/intelligence/finops_os.rs` (or extend `finops.rs`) + handlers
2. Dashboard panel(s) with `data-testid`, route wiring, Command Palette hooks
3. `web/dashboard/tests/ai-os-v13.spec.ts`
4. Mark phases 75–84 **Ship** in `PHASES-AI-OS.md` + ROADMAP v13 section

## Deploy

**Live** on `212.8.252.194:30090` (remote Linux build — do **not** use `--local-build` from macOS).

```bash
./scripts/deploy-remote.sh 212.8.252.194 sus   # remote cargo on Linux
./scripts/post-deploy-verify.sh 212.8.252.194 sus
# UI: http://212.8.252.194:30090
```

## Local dev after reboot

```bash
cargo build --release
cd web/dashboard && npm run build && cd ../..
cargo build --release   # embed dashboard dist
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: cd web/dashboard && npm run test:e2e -- tests/ai-os-v*.spec.ts
```
