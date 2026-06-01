# Aether Dashboard — Liquid Glass Design System

The dashboard uses a **Liquid Glass** visual language: layered gradients, backdrop blur, and composable CSS tokens defined in [`src/index.css`](../src/index.css).

## Hierarchy

| Layer | Class | Use |
|-------|-------|-----|
| Page backdrop | `app-shell`, body gradients | Full-viewport ambient color |
| Hero shell | `.command-center-shell` | Overview command center, primary hero blocks |
| Section shell | `.overview-section-shell` | Page main content wrapper (all routes) |
| Panel card | `.glass-panel-card` | Cards, list items, split panes |
| Metric | `.glass-metric-card` | Stat cards, KPI tiles |
| Toolbar | `.glass-toolbar` | PageToolbar, filter bars |
| Footer | `.glass-footer` | Dashboard footer chrome |
| Composable | `.glass-input`, `.glass-select`, `.glass-table-shell`, `.glass-drawer`, `.glass-modal-panel`, `.glass-code-block`, `.glass-empty-state`, `.glass-dropdown-surface` | Forms, tables, drawers, modals, code, empty states, menus |

## Composable micro-tokens (Sweep 8)

| Class | Replaces | Use |
|-------|----------|-----|
| `.glass-divider` | `border-slate-*` color only | Shared border color for panels |
| `.glass-divider-b` / `.glass-divider-t` | `border-b/t border-slate-*` | Section separators |
| `.glass-inset-surface` | `bg-[#0B0E14]`, `bg-slate-800/*` fills | Nested inset blocks, chat bubbles, code overlays |
| `.glass-tab` / `.glass-tab-active` | Inline tab pills | Aliases for `.tab-chip` / `.tab-chip-active` |
| `.glass-nav-item` | `hover:bg-slate-800/*` | Navbar mobile drawer rows |
| `.glass-progress-track` | `bg-slate-800` troughs | Progress / attestation bars |
| `.glass-row-hover` | Table row hover fills | `<tr>` hover in data tables |
| `.glass-inset-hover` | Button/icon hover on inset surfaces | Secondary controls |

### Migration examples

```tsx
// Divider inside a panel
<div className="glass-divider-b pb-4">…</div>

// Form field — prefer glass-input over hex insets
<input className="glass-input" />

// Table row
<tr className="glass-table-row">…</tr>

// Tabs
<button className={`glass-tab ${active ? 'glass-tab-active' : ''}`}>Overview</button>
```

## React primitive

[`GlassSection`](../src/components/GlassSection.tsx) wraps panels with optional label/title/subtitle and accent colors (`blue`, `purple`, `red`, `neutral`). Variants: `hero`, `section`, `panel`.

## Themes

Three themes via [`ThemeContext`](../src/contexts/ThemeContext.tsx):

- **Dark** (default) — blue/orange accents, deep navy gradients
- **Steel** — cool gray industrial palette (`.steel-theme` overrides in CSS)
- **Aurora** — violet/cyan accents (`.aurora-theme` overrides)

Steel and Aurora override shell, panel, inset, divider, input, and toolbar tokens — not individual inline classes.

## Conventions

- Prefer token classes over inline `bg-slate-*`, hex surfaces (`bg-[#…]`), or `border-slate-*` for chrome.
- `text-slate-*` remains acceptable for typography hierarchy.
- Semantic status colors (`emerald`, `amber`, `red`, status dots) may use Tailwind directly.
- Preserve all `data-testid` attributes when restyling.
- Navbar uses `navbar-blur` / `navbar-glass-accent` from [`themeSurface.ts`](../src/utils/themeSurface.ts).
- Modals: `.glass-modal-backdrop` + `.glass-modal-panel`.

## Verification

```bash
cd web/dashboard && npm run test:e2e -- tests/liquid-glass-smoke.spec.ts
node scripts/check-no-hex-surfaces.mjs
```

Smoke tests assert `.overview-section-shell` (or `.command-center-shell`) is visible and `backdrop-filter` is not `none` on all 41 dashboard routes (dark default plus steel/aurora theme sweeps). The guard script runs as part of `npm run build` and is also invoked by `make ci` via the `dashboard-glass` target.
