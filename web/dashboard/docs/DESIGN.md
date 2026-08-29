# Aether Dashboard — Aurora / Apple Design System

The dashboard follows **Aurora’s Apple.com anatomy** with **Apple blue** accents from Aurora `globals.css` (not Zyvor marketing orange).

## Hierarchy

| Layer | Class / component | Use |
|-------|-------------------|-----|
| Page backdrop | `app-shell`, `--background` | Full-viewport canvas (light-first) |
| Global nav | `GlobalNav` (44px sticky blur) | All destinations via top links + flyouts |
| Page hero | `PageHero` / `AuroraPage` | Eyebrow, title, lead, actions, stats |
| Surfaces | `.glass`, `Card` | Flat Apple cards (not Liquid Glass gradients) |
| Forms | `.login-*`, `Input`, `Button` (pill) | Aurora login + console controls |

## Brand

- Primary: Apple blue `#0071e3` (light) / `#0a84ff` (dark)
- Neutrals: Apple `#ffffff` / `#f5f5f7` / `#1d1d1f`
- Display: Archivo wordmark; body SF / system
- Layout spacing: zyvor-web nav anatomy (44px bar, 1180px max width)

## Auth (demo)

Default local login (disable with `AETHER_DEMO_AUTH=0`):

- Username: `admin`
- Password: `Admin@321`

Override: `AETHER_DEMO_USER` / `AETHER_DEMO_PASSWORD`.

## Themes

`light` | `dark` | `system` via ThemeContext. No accent swatch picker.

## Buttons

- Primary: pill, 44px min height, blue fill, hover lift
- Secondary: pill outline blue, tint wash on hover
- Tertiary: text + tint for toolbars
- Legacy `.btn-primary` / `.btn-secondary` mirror `Button` variants

## Conventions

- Prefer `AuroraPage` + `Card` over Liquid Glass shells
- Preserve all `data-testid` attributes when restyling
- Nav: `data-testid="global-nav"`
- Semantic warning colors (amber/orange for degraded health, log WARN) are status-only—not brand

## Verification

```bash
cd web/dashboard && npm run check:hex-surfaces && npm run build
npm run test:e2e -- tests/aurora-shell-smoke.spec.ts
```
