# Dashboard E2E tests

Playwright specs live in `tests/`. CI runs the full suite via the `dashboard-e2e` job after building the dashboard bundle and a release `aether` binary.

## Run locally

```bash
cd web/dashboard
npm run build
cd ../.. && cargo build --release
cd web/dashboard
npx playwright install chromium
npx playwright test
```

The config starts `cargo run -- serve` on port `5090` unless `AETHER_E2E_SKIP_SERVER=1` and a server is already running.

## Manual SSE reconnect smoke

Automated coverage of the amber reconnect banner is intentionally avoided in CI (10s timer + live SSE are flaky). To verify manually:

1. Start `aether serve` and open the dashboard.
2. Block or kill the SSE stream (e.g. stop the server for ~15s, then restart).
3. Confirm the disconnect toast appears after ~10s and the amber **Live updates disconnected** banner shows with **Refresh now**.
4. Restore SSE and confirm the banner clears and a reconnect toast appears if offline for 10s+.

## PR screenshots (suggested)

- Onboarding strip on empty Overview
- Help → About dialog
- API-down retry UI (GitOps or Policy)
- Light-theme deploy modal (`Theme` → Light, `/workloads?deploy=1`)
