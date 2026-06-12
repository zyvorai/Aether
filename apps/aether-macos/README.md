# Aether macOS Shell (Tauri)

Native macOS wrapper for the Aether AI Infrastructure OS dashboard — **Ship** tier with notarized DMG CI (`.github/workflows/macos-dmg.yml`).

## Features

- **Menu bar tray** — Open Aether, Command Palette, Quit
- **⌘K global shortcut** — Spotlight-style bridge to the dashboard command palette (works when Aether is focused; registered as system shortcut on macOS)
- **Dock reopen** — Clicking the dock icon restores the main window
- **Frosted window** — Transparent overlay title bar (`titleBarStyle: Overlay`)

## Prerequisites

- Rust toolchain (same as main Aether repo)
- Node.js 20+
- macOS 12+

```bash
cd apps/aether-macos
npm install
```

## Development

1. Start the Aether API + embedded dashboard:

```bash
cargo run -- serve
```

2. Run the native shell (live dashboard URL recommended):

```bash
cd apps/aether-macos
AETHER_DASHBOARD_URL=http://127.0.0.1:5090 npm run dev
```

The web dashboard listens for `aether-open-command-palette` and exposes `window.__AETHER_OPEN_COMMAND_PALETTE__()` for native bridges.

## Production build

```bash
cd web/dashboard && npm run build
cd ../../apps/aether-macos
npm run build
```

Output: `src-tauri/target/release/bundle/dmg/`

## Related

- [VISION-AI-OS.md](../../docs/VISION-AI-OS.md)
- [WEBUI.md](../../docs/WEBUI.md)
