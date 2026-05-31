# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-31 · Resume with **"cont"** to ship the next batch.

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced with `origin/main` after v13 push) |
| Commit | v13 Era H — `feat(ai-os): ship FinOps & cost platform v13 (phases 75–84)` |
| Uncommitted | `web/dashboard/node_modules/*` only (safe to discard) |

## Shipped (v1–v13)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v1–v4 | Core panels | 1–4 | Command Center, Copilot, topology, intent pipeline |
| v5–v7 | A–B | 5–24 | Autonomous execute, agent loop, Intent platform |
| v8 | C Multi-Cloud | 25–34 | `federation_os.rs`, federation panels |
| v9 | D SRE | 35–44 | `sre_os.rs`, SRE reliability panel |
| v10 | E Knowledge Graph | 45–54 | `graph_os.rs`, graph platform |
| v11 | F macOS Native | 55–64 | `macos_os.rs`, `MacOSPlatformPanel` |
| v12 | G Copilot & LLM | 65–74 | `copilot_os.rs`, `CopilotPlatformPanel` |
| v13 | H FinOps & Cost | 75–84 | `finops_os.rs`, `FinOpsPlatformPanel` |

Docs: `docs/PHASES-AI-OS.md`, `docs/ROADMAP.md` (eras marked **Ship** through phase 84).

## Next batch — v14 Era I (phases 85–94)

See `docs/PHASES-AI-OS.md` § Era I for the next era scope.

**Implementation pattern (repeat per batch):**

1. `src/intelligence/*_os.rs` + API handlers
2. Dashboard panel(s) with `data-testid`
3. `web/dashboard/tests/ai-os-v14.spec.ts`
4. Mark phases **Ship** in `PHASES-AI-OS.md` + ROADMAP

## Local dev

```bash
cargo build --release
cd web/dashboard && npm run build && cd ../..
cargo build --release
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: cd web/dashboard && npm run test:e2e -- tests/ai-os-v13.spec.ts
```
