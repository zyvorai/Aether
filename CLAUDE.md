# Aether Development Notes

## Project Overview

Aether is a Universal Runtime Control Plane written in Rust. It deploys workloads to Podman, Kubernetes, KubeVirt, and Metal3 from a single YAML specification.

## Build & Test

```bash
cargo build          # Debug build
cargo build --release  # Release build
cargo test           # Run all 916 tests
cargo clippy         # Lint (must pass with zero warnings)
cargo check          # Fast type-check
make ci              # Full CI pipeline
```

## Project Structure

- `src/main.rs` — CLI entrypoint, clap parser, output mode setup
- `src/cli.rs` — CLI argument definitions (40+ subcommands)
- `src/commands.rs` — All command handler implementations (~4500 lines)
- `src/spec.rs` — Workload YAML schema (Workload struct, validation)
- `src/engine.rs` — Runtime decision engine (GPU→KubeVirt, high resources→Metal3, etc.)
- `src/adapters/` — Runtime implementations (podman.rs, kube.rs, kubevirt.rs, metal.rs)
- `src/migration.rs` — Migration engine (Immediate, Blue-Green, Rolling)
- `src/api/` — Axum REST API server (mod.rs, handlers.rs, types.rs)
- `src/state.rs` — Local state store (atomic writes, file locking)
- `src/secrets.rs` — AES-256-GCM encryption, rotation policies
- `src/audit.rs` — Audit trail with SHA-256 integrity hashes
- `src/output.rs` — Terminal formatting (colors, tables, spinners)
- `src/ui/` — TUI dashboard (ratatui)

## Key Patterns

- **Runtime trait** — `src/runtime.rs` defines `build/run/stop/status/logs/delete/list`
- **Atomic state** — Write to `.tmp`, rename (crash-safe), with `flock` advisory locking
- **Error handling** — `anyhow::Result` everywhere, context with `.context()/.bail!()`
- **Async** — Tokio runtime, `async_trait` for runtime adapters
- **CLI context** — `src/commands.rs::ctx` module holds session-scoped globals

## Security Architecture

- `AETHER_SECRET_KEY` env → SHA-256 → AES-256-GCM encryption
- `AETHER_API_KEY` env → Bearer token authentication on API
- Backups/snapshots: `0o600` permissions
- K8s: 5-min timeouts, resource cleanup on failure, 409 conflict handling
- Input: DNS-1123 name validation, path traversal prevention

## Testing

- 916 tests (826 lib + 46 bin + 44 integration)
- Tests use `tempfile::tempdir()` for isolated filesystem state
- No external services needed (K8s/Podman tests are unit tests against manifest generation)
- `#[tokio::test]` for async command tests

## Default Ports

- API server: `5090` (configurable via `--port`)
- Brand color: Dark Orange `#d35400` / RGB `(211, 84, 0)`
