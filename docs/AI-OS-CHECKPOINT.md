# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-31 · Resume with **"cont"** to ship the next batch.

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced with `origin/main` after v14 push) |
| Commit | v14 Era I — `feat(ai-os): ship security & compliance platform v14 (phases 85–94)` |
| Uncommitted | `web/dashboard/node_modules/*` only (safe to discard) |

## Shipped (v1–v14)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v12 | G Copilot & LLM | 65–74 | `copilot_os.rs`, `CopilotPlatformPanel` |
| v13 | H FinOps & Cost | 75–84 | `finops_os.rs`, `FinOpsPlatformPanel` |
| v14 | I Security & Compliance | 85–94 | `security_os.rs`, `SecurityPlatformPanel` |

Docs: `docs/PHASES-AI-OS.md`, `docs/ROADMAP.md` (eras marked **Ship** through phase 94).

## Next batch — v15 Era J (phases 95–104)

See `docs/PHASES-AI-OS.md` § Era J for scope.

## Local dev

```bash
cargo build --release
cd web/dashboard && npm run build && cd ../..
cargo build --release
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: cd web/dashboard && npm run test:e2e -- tests/ai-os-v14.spec.ts
```
