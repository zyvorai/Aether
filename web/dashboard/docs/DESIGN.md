# Aether Dashboard — Aurora / Apple Design System

The dashboard follows **Aurora’s Apple.com anatomy** with **Apple blue** accents from Aurora `globals.css` (not Zyvor marketing orange).

## Hierarchy

| Layer | Class / component | Use |
|-------|-------------------|-----|
| Page backdrop | `app-shell`, `--background` | Full-viewport canvas (light-first paper `#f5f5f7`) |
| Global nav | `GlobalNav` (44px sticky blur) | All destinations via top links + flyouts |
| Page hero | `PageHero` / `AuroraPage` | Eyebrow → `DisplayTitle` → lead → actions |
| Highlights | `AppleHighlightsRow` + `.apple-chapter-dark` | Ink “Get the highlights.” band (iPad Pro chapter) |
| Surfaces | `.glass`, `Card` | Flat Apple cards (not Liquid Glass gradients) |
| Forms | `.login-*`, `Input`, `Button` (pill / 980px) | Apple Store paper login + console controls |

## Brand

- Primary: Apple blue `#0071e3` (light) / `#0a84ff` (dark)
- Neutrals: Apple `#ffffff` / `#f5f5f7` / `#1d1d1f`
- Aliases: `--apple-ink` / `--apple-paper` / `--apple-action` / `--font-hero` in `theme.css`
- Display: Archivo / SF Pro; body SF / system
- Layout spacing: zyvor-web nav anatomy (44px bar, 1180px max width)

## Auth (demo)

Default local login (disable with `AETHER_DEMO_AUTH=0`):

- Username: `admin`
- Password: `Admin@321`

Override: `AETHER_DEMO_USER` / `AETHER_DEMO_PASSWORD`.

Login (`LoginGate`) is **Apple Store paper** (white/light chapters + pill Sign in) — not a dark mesh hero.

## Themes

`light` | `dark` | `system` via ThemeContext. No accent swatch picker.
Dark chapters (`.apple-chapter-dark`) use ink `#1d1d1f` in light theme and pure black under `.dark-theme`.

## Buttons

- Primary: pill (`--radius-pill` / ~980px), 44px min height, blue fill
- Secondary: pill outline blue, tint wash on hover
- Tertiary: text + tint for toolbars
- Legacy `.btn-primary` / `.btn-secondary` mirror `Button` variants

## Conventions

- Prefer `AuroraPage` + `Card` over Liquid Glass shells
- Pass `stats` to `AuroraPage` / `PageHero` when a page has headline metrics — they render as an ink highlights band
- Overview (`/`) also ships an ink highlights row for workloads / health / clusters
- Preserve all `data-testid` attributes when restyling
- Nav: `data-testid="global-nav"`
- Hex surfaces stay in CSS/`theme.css` only — `npm run check:hex-surfaces` fails on `bg-[#…]` / `bg-black` in TSX
- Semantic warning colors (amber/orange for degraded health, log WARN) are status-only—not brand

## Verification

```bash
cd web/dashboard && npm run check:hex-surfaces && npm run build
npm run test:e2e -- tests/aurora-shell-smoke.spec.ts
```
