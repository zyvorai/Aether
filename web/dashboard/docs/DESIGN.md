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
| Composable | `.glass-input`, `.glass-select`, `.glass-table-shell`, `.glass-drawer`, `.glass-modal-panel`, `.glass-code-block`, `.glass-empty-state`, `.glass-dropdown-surface` | Forms, tables, drawers, modals, code, empty states, menus |

## React primitive

[`GlassSection`](../src/components/GlassSection.tsx) wraps panels with optional label/title/subtitle and accent colors (`blue`, `purple`, `red`, `neutral`). Variants: `hero`, `section`, `panel`.

## Themes

Three themes via [`ThemeContext`](../src/contexts/ThemeContext.tsx):

- **Dark** (default) — blue/orange accents, deep navy gradients
- **Steel** — cool gray industrial palette (`.steel-theme` overrides in CSS)
- **Aurora** — violet/cyan accents (`.aurora-theme` overrides)

Steel and Aurora override shell, panel, input, and toolbar tokens — not individual inline classes.

## Conventions

- Prefer token classes over inline `bg-slate-*` or hex surfaces.
- Preserve all `data-testid` attributes when restyling.
- Navbar uses `navbar-blur` / `navbar-glass-accent` from [`themeSurface.ts`](../src/utils/themeSurface.ts).
- Modals: `.glass-modal-backdrop` + `.glass-modal-panel`.

## Verification

```bash
cd web/dashboard && npm run test:e2e -- tests/liquid-glass-smoke.spec.ts
```

Smoke tests assert `.overview-section-shell` (or `.command-center-shell`) is visible and `backdrop-filter` is not `none` across routes and themes.
