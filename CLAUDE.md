# Aether Development Notes

## Project Overview

Aether is a Universal Runtime Control Plane written in Rust. It deploys workloads to Podman, Kubernetes, KubeVirt, and Metal3 from a single YAML specification.

## Build & Test

```bash
cargo build          # Debug build
cargo build --release  # Release build
cargo test           # Run all 1,325 tests
cargo clippy         # Lint (must pass with zero warnings)
cargo check          # Fast type-check
make ci              # Full CI pipeline
```

## Project Structure

- `src/main.rs` — CLI entrypoint, clap parser, output mode setup
- `src/cli.rs` — CLI argument definitions (40+ subcommands)
- `src/commands.rs` — All command handler implementations (~4500 lines)
- `src/spec.rs` — Workload YAML schema (Workload struct, validation, intent types)
- `src/engine.rs` — Runtime decision engine (GPU→KubeVirt, high resources→Metal3, etc.)
- `src/adapters/` — Runtime implementations (podman.rs, docker.rs, kube.rs, kubevirt.rs, metal.rs)
- `src/migration.rs` — Migration engine (Immediate, Blue-Green, Rolling)
- `src/api/` — Axum REST API server (mod.rs, handlers.rs, types.rs)
- `src/rbac.rs` — RBAC key store (Admin/Operator/Viewer roles, API middleware enforcement)
- `src/gitops.rs` — GitOps reconciliation loop
- `src/helm.rs` — Helm chart generation and management
- `src/state.rs` — Local state store (atomic writes, file locking)
- `src/secrets.rs` — AES-256-GCM encryption, rotation policies
- `src/audit.rs` — Audit trail with SHA-256 integrity hashes
- `src/output.rs` — Terminal formatting (colors, tables, spinners)
- `src/ui/` — TUI dashboard (ratatui)
- `web/dashboard/` — React 18 + TypeScript + Tailwind CSS web dashboard (Vite)
- `packaging/aether.service` — Systemd unit file with security hardening

## Key Patterns

- **Runtime trait** — `src/runtime.rs` defines `build/run/stop/status/logs/delete/list/update/capacity`
- **Atomic state** — Write to `.tmp`, rename (crash-safe), with `flock` advisory locking
- **Error handling** — `anyhow::Result` everywhere, context with `.context()/.bail!()`
- **Async** — Tokio runtime, `async_trait` for runtime adapters
- **CLI context** — `src/commands.rs::ctx` module holds session-scoped globals
- **Intent engine** — `intent:` YAML field drives runtime scoring weights, filtering, and score multipliers; violations tracked in reconciliation loop
- **Structured logging** — `AETHER_LOG_FORMAT=json` enables JSON-formatted tracing output
- **Rate limiting** — Tower middleware limits API to 200 concurrent requests
- **RBAC** — API middleware enforces Admin/Operator/Viewer roles via RbacStore; falls back to `AETHER_API_KEY`
- **SSE events** — API handlers emit `ServerEvent` after mutations; web dashboard receives real-time updates via `useEventStream`
- **Health check loop** — Background tokio task runs health checks every 30s when `aether serve` is running
- **KubeVirt live migration** — `kubevirt.liveMigration` spec renders `evictionStrategy: LiveMigrate` + masquerade pod networking (bridge is not migratable); `aether live-migrate <name>` creates/watches a VirtualMachineInstanceMigration; passthrough GPUs + liveMigration is rejected at validation (use `requirements.gpu.vgpuProfile` for mediated vGPU slices)

## Security Architecture

- `AETHER_SECRET_KEY` env → SHA-256 → AES-256-GCM encryption
- `AETHER_API_KEY` env → Bearer token authentication on API (backward-compat fallback)
- **RBAC API** — `RbacStore` enforces Admin/Operator/Viewer roles; endpoints: `GET/POST /api/rbac/keys`, `POST /api/rbac/keys/revoke`
- Backups/snapshots: `0o600` permissions
- K8s: 5-min timeouts, resource cleanup on failure, 409 conflict handling
- Input: DNS-1123 name validation, path traversal prevention
- Email SMTP notifications via `lettre` crate (`ChannelType::Email`)

## Testing

- 1,325 tests (1,203 lib + bin + integration)
- Tests use `tempfile::tempdir()` for isolated filesystem state
- No external services needed (K8s/Podman tests are unit tests against manifest generation)
- `#[tokio::test]` for async command tests
- Multi-cluster tests (6 unit tests for namespace/context resolution logic)

## State and Lock File Paths

- **State file**: `~/.aether/state.json` (persistent, shared between CLI and systemd service)
- **Lock file (root)**: `/run/aether/`
- **Lock file (user)**: `$XDG_RUNTIME_DIR/aether/`
- **Lock file (fallback)**: `/tmp/aether-<uid>`

## Web Dashboard

- React 18 + TypeScript + Tailwind CSS + Vite (`web/dashboard/`)
- SSE real-time updates via `useEventStream` hook
- WorkloadDetail panel with tabbed view (Overview/Logs/Drift/Scoring) and action buttons (Start/Stop/Restart/Delete)
- LogViewer with auto-polling, follow mode, filter, line numbers, color-coded log levels, copy-to-clipboard
- Command Palette (Cmd+K / Ctrl+K) with fuzzy search across pages, workloads, and actions
- Intent Debugger with AI scoring visualization and pure SVG radar chart
- SSE connection indicator (green/red dot in navbar)
- Keyboard shortcuts: `r` (refresh), `?` (command palette), `Cmd+K` (command palette)

## Default Ports

- API server: `5090` (configurable via `--port`)
- Brand color: Dark Orange `#d35400` / RGB `(211, 84, 0)`
