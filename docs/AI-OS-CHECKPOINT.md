# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-31 · **144 phases shipped (v1–v19).**

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced after v19 push) |
| Commit | v19 Era N — `feat(ai-os): ship live labs & reference cluster v19 (phases 135–144)` |

## Shipped (v1–v19, phases 1–144)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v17 | L Extensions & Native | 115–124 | `extensions_os.rs` |
| v18 | M Production & Trust | 125–134 | `production_os.rs` |
| v19 | N Live Labs & Reference Cluster | 135–144 | `livelabs_os.rs` |

## Local dev

```bash
cargo build --release
cd web/dashboard && npm run build && cd ../..
cargo build --release
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: cd web/dashboard && npm run test:e2e -- tests/ai-os-v19.spec.ts
# Live labs: AETHER_LABS_LIVE=1 make reference-cluster-live
```
