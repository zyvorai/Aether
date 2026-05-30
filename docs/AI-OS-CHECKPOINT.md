# AI OS Backlog — Reboot Checkpoint

> Saved: 2026-05-30 · Resume with **"cont"** to ship the next batch.

## Git state

| Item | Value |
|------|-------|
| Branch | `main` (synced with `origin/main`) |
| Commit | `2c36766` — `feat(ai-os): ship intelligence backlog v1–v11 through macOS native OS` |
| Uncommitted | `web/dashboard/node_modules/*` only (safe to discard: `git checkout -- web/dashboard/node_modules`) |

## Shipped (v1–v11)

| Batch | Era | Phases | Key modules |
|-------|-----|--------|-------------|
| v1–v4 | Core panels | 1–4 | Command Center, Copilot, topology, intent pipeline |
| v5–v7 | A–B | 5–24 | Autonomous execute, agent loop, Intent platform |
| v8 | C Multi-Cloud | 25–34 | `federation_os.rs`, federation panels |
| v9 | D SRE | 35–44 | `sre_os.rs`, SRE reliability panel |
| v10 | E Knowledge Graph | 45–54 | `graph_os.rs`, graph platform |
| v11 | F macOS Native | 55–64 | `macos_os.rs`, Tauri shell, `MacOSPlatformPanel` |

Docs: `docs/PHASES-AI-OS.md`, `docs/ROADMAP.md` (eras marked **Ship** through phase 64).

## Next batch — v12 Era G (phases 65–74)

**Copilot & LLM** — see `docs/PHASES-AI-OS.md` § Era G.

| # | Phase | Focus |
|---|-------|-------|
| 65 | Copilot tool confirmation UX | Batch approve pending actions |
| 66 | Voice copilot | Speech → infra queries (Lab) |
| 67 | Copilot memory | Session + fleet context |
| 68 | Multi-agent copilot | SRE / FinOps sub-agents |
| 69 | LLM intent parsing | Optional OpenAI/Anthropic backend |
| 70 | Copilot runbook author | NL → markdown runbooks |
| 71 | Copilot policy explainer | OPA violation plain English |
| 72 | Copilot in terminal | `aether copilot` TUI mode |
| 73 | Copilot audit trail | All NL actions logged |
| 74 | Copilot RBAC scopes | Role-limited tool access |

**Implementation pattern (repeat per batch):**

1. `src/intelligence/copilot_os.rs` (or extend `src/copilot/`) + handlers in `src/api/intelligence_handlers.rs`
2. Dashboard panel(s) with `data-testid`, route wiring, Command Palette hooks
3. `web/dashboard/tests/ai-os-v12.spec.ts`
4. Mark phases 65–74 **Ship** in `PHASES-AI-OS.md` + ROADMAP v12 section

## Deploy

**Live** on `212.8.252.194:30090` (commit `2c36766`, remote Linux build — do **not** use `--local-build` from macOS).

```bash
./scripts/deploy-remote.sh 212.8.252.194 sus   # remote cargo on Linux
./scripts/post-deploy-verify.sh 212.8.252.194 sus
# UI: http://212.8.252.194:30090
```

## CI note

Push `2c36766` triggered [CI run 26684161065](https://github.com/ssahani/Aether/actions/runs/26684161065) — **Dashboard build** and **Security Audit** jobs failed in ~3s (likely Actions quota/billing, not compile). Re-run after reboot or fix billing.

## Local dev after reboot

```bash
cargo build --release
cd web/dashboard && npm run dev   # or npm run build
AETHER_MOCK_IDP=1 cargo run -- serve --port 5090
# E2E: npm run test:e2e -- tests/ai-os-v*.spec.ts
```
