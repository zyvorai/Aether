# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-31 · **124 phases shipped (v1–v17).**

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced after v17 push) |
| Commit | v17 Era L — `feat(ai-os): ship extensions & native v17 (phases 115–124)` |

## Shipped (v1–v17, phases 1–124)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v15 | J Platform & Ecosystem | 95–104 | `platform_os.rs` |
| v16 | K Lab Graduation | 105–114 | `labs_os.rs` |
| v17 | L Extensions & Native | 115–124 | `extensions_os.rs` |

Docs: `docs/PHASES-AI-OS.md`, `docs/ROADMAP.md` — phases 1–124 tracked.

## Local dev

```bash
cargo build --release
cd web/dashboard && npm run build && cd ../..
cargo build --release
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: cd web/dashboard && npm run test:e2e -- tests/ai-os-v17.spec.ts
```
