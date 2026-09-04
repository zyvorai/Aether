# Aether Development Notes

## Project Overview

Aether is a Universal Runtime Control Plane written in Rust. It deploys workloads to Podman, Kubernetes, KubeVirt, and Metal3 from a single YAML specification.

## Build & Test

```bash
cd web/dashboard && npm run build   # Build embedded UI assets (see note below)
cargo build          # Debug build
cargo build --release  # Release build
cargo test           # Run all 1,329 tests
cargo clippy --all-targets --all-features  # Lint (must pass with zero warnings)
cargo check          # Fast type-check
make ci              # Full CI pipeline (tests, lint, dashboard build, vitest)
```

`web/dashboard/dist/` is generated, not committed. `src/api/handlers.rs` embeds it with `include_str!`, so `build.rs` runs the dashboard build when the assets are missing and writes placeholder stubs if npm is unavailable. Run a real `npm run build` before shipping a binary.

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
- **RBAC** — API middleware enforces Admin/Operator/Viewer roles via RbacStore; falls back to `AETHER_API_KEY`. Viewers get read-only GETs except privileged GETs (`/api/cluster/ws/exec`), which require Operator or Admin
- **Cluster discovery logs/exec** — `/api/cluster/logs` and `/api/cluster/ws/exec` serve discovered pods and KubeVirt VMs; VM lookups try both `kubevirt.io/vm=` and `vm.kubevirt.io/name=` and prefer a Running virt-launcher pod
- **Honest apply semantics** — intelligence execute/apply endpoints (remediation, FinOps, security, capacity, evolution, federation, GitOps agent, game-day) report `skipped` with "not implemented" instead of claiming success when no mutation is wired
- **SSE events** — API handlers emit `ServerEvent` after mutations; web dashboard receives real-time updates via `useEventStream`
- **Health check loop** — Background tokio task runs health checks every 30s when `aether serve` is running
- **Deployment identity** — `GET /health` (unauthenticated, root path not `/api`) returns `hostname` (auto-detected, cached) and `environment` (from `AETHER_ENVIRONMENT_NAME`, e.g. `"Lab · k3s-212"`); the dashboard shows this as a small tone-colored badge in `GlobalNav` and on `LoginGate` so a user with multiple Aether deployments bookmarked knows which one they're on
- **KubeVirt live migration** — `kubevirt.liveMigration` spec renders `evictionStrategy: LiveMigrate` + masquerade pod networking (bridge is not migratable); `aether live-migrate <name>` creates/watches a VirtualMachineInstanceMigration; passthrough GPUs + liveMigration is rejected at validation (use `requirements.gpu.vgpuProfile` for mediated vGPU slices)

## Security Architecture

- `AETHER_SECRET_KEY` env → SHA-256 → AES-256-GCM encryption
- `AETHER_API_KEY` env → Bearer token authentication on API (backward-compat fallback)
- **RBAC API** — `RbacStore` enforces Admin/Operator/Viewer roles; endpoints: `GET/POST /api/rbac/keys`, `POST /api/rbac/keys/revoke`
- **Privileged GETs** — `rbac::check_permission` denies Viewers the cluster exec WebSocket so a read-only key cannot open a pod shell
- **Local key material** — `certs/`, `keys/`, `*.pem`, `*.key` are gitignored (mock-idp fixtures excepted); never commit TLS or signing keys
- Backups/snapshots: `0o600` permissions
- K8s: 5-min timeouts, resource cleanup on failure, 409 conflict handling
- Input: DNS-1123 name validation, path traversal prevention
- Email SMTP notifications via `lettre` crate (`ChannelType::Email`)

## Testing

- 1,329 tests (lib + integration)
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
- Discovered pods/VMs: WorkloadDetail offers Logs plus a Shell for `Pod`, `Deployment`, `StatefulSet`, `DaemonSet`, `VirtualMachine`, and `VirtualMachineInstance`; `pickExecPodName` (`src/utils/clusterExec.ts`) resolves the backing pod before the exec WebSocket opens
- Unit tests: `npm run test` (vitest, `src/**/*.test.ts`); e2e: `npm run test:e2e`

## Default Ports

- API server: `5090` (configurable via `--port`)
- Brand color: Apple blue `#0071e3` light / `#0a84ff` dark (Aurora — product console; zyvor-web orange is marketing-only)
- Visual shell: Apple.com chapters — `PageHero` + `AppleHighlightsRow` ink bands; login is Store paper (see `web/dashboard/docs/DESIGN.md`)
- Hex surfaces only in CSS/`theme.css`; run `npm run check:hex-surfaces` before build
