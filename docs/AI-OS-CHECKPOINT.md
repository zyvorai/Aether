# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-31 · **114 phases shipped (v1–v16).**

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced after v16 push) |
| Commit | v16 Era K — `feat(ai-os): ship lab graduation v16 (phases 105–114)` |

## Shipped (v1–v16, phases 1–114)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v14 | I Security & Compliance | 85–94 | `security_os.rs` |
| v15 | J Platform & Ecosystem | 95–104 | `platform_os.rs`, `/v1/intelligence/*` |
| v16 | K Lab Graduation | 105–114 | `labs_os.rs`, `/api/intelligence/labs/*` |

Docs: `docs/PHASES-AI-OS.md`, `docs/ROADMAP.md` — phases 1–114 tracked.

## Local dev

```bash
cargo build --release
cd web/dashboard && npm run build && cd ../..
cargo build --release
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: cd web/dashboard && npm run test:e2e -- tests/ai-os-v16.spec.ts
```
