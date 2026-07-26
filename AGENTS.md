# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the Rust application: CLI entrypoints in `main.rs`/`cli.rs`, shared library code in `lib.rs`, runtime adapters in `src/adapters/`, API handlers in `src/api/`, AI/scoring logic in `src/ai/`, and terminal UI code in `src/ui/`. Integration tests live in `tests/`. The web dashboard is isolated in `web/dashboard/` with React + TypeScript sources under `web/dashboard/src/`. Deployment assets and reference material live in `templates/`, `examples/`, `helm/aether/`, `packaging/`, and `docs/`.

## Build, Test, and Development Commands
Use `make build` for a debug Rust build and `make release` for an optimized binary. Run `make test` for the full Rust test suite and `make lint` for `cargo fmt --check` plus strict `clippy`. `make ci` runs the expected pre-PR checks (`dashboard-glass`: hex/black surface guard + dashboard build, and `dashboard-unit`: vitest). For quick spec validation, use `make validate` or `cargo run --release -- validate --spec workload.yaml`. For the dashboard, run `cd web/dashboard && npm run dev` for local development and `npm run build` for a production bundle. Dashboard styling uses Liquid Glass tokens in `web/dashboard/src/index.css`; run `npm run check:hex-surfaces` and `npm run test:e2e -- tests/liquid-glass-smoke.spec.ts` before UI changes.

`web/dashboard/dist/` is generated and gitignored, but the Rust binary embeds it via `include_str!`. `build.rs` runs the dashboard build when those assets are missing and falls back to placeholder stubs when npm is unavailable, so always run a real `npm run build` before producing a shippable binary. Never commit TLS or signing material: `certs/`, `keys/`, `*.pem`, and `*.key` are gitignored.

## Coding Style & Naming Conventions
Follow Rust 2021 defaults and let `rustfmt` drive formatting; do not hand-format around it. Keep modules and functions in `snake_case`, types and traits in `PascalCase`, and prefer small focused modules over large mixed-responsibility files. In the dashboard, keep React components in `PascalCase.tsx`, hooks in `useX.ts`, and shared helpers in `src/utils/`. Use clear field names that match workload and API terminology already used across `schema/` and `src/api/types.rs`.

## Testing Guidelines
Add unit tests next to Rust code with `#[cfg(test)]` when behavior is local, and use `tests/` for cross-module workflows. Run `cargo test --verbose` before opening a PR. When changing validation, scheduling, or runtime behavior, add or update fixture-driven checks using the sample workload YAML files in the repo root or `examples/`. Dashboard changes should at least build cleanly with `npm run build`; pure logic belongs in `src/utils/` with a matching `*.test.ts` covered by `npm run test` (vitest runs in a Node environment, so keep React out of unit tests).

## Commit & Pull Request Guidelines
Git history uses Conventional Commit prefixes such as `feat:`, `fix:`, and `docs:`; keep that format and make scopes descriptive. PRs should summarize the behavior change, list verification steps, link related issues, and call out docs or schema updates. Include screenshots or short recordings for `web/dashboard/` UI changes.

## Security & Configuration Tips
Do not commit secrets, kubeconfigs, or generated state such as `.aether/` contents. Prefer environment variables like `AETHER_NAMESPACE` and `AETHER_API_KEY` for local configuration, and sanitize any example manifests before committing them.
