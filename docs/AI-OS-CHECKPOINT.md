# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-31 · **134 phases shipped (v1–v18).**

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced after v18 push) |
| Commit | v18 Era M — `feat(ai-os): ship production & trust v18 (phases 125–134)` |

## Shipped (v1–v18, phases 1–134)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v16 | K Lab Graduation | 105–114 | `labs_os.rs` |
| v17 | L Extensions & Native | 115–124 | `extensions_os.rs` |
| v18 | M Production & Trust | 125–134 | `production_os.rs` |

## Local dev

```bash
cargo build --release
cd web/dashboard && npm run build && cd ../..
cargo build --release
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: cd web/dashboard && npm run test:e2e -- tests/ai-os-v18.spec.ts
```
