# Secure Dark Professional — Aether Dashboard

Design direction for the Aether universal runtime control plane: trustworthy, sharp, enterprise-grade dark UI with **solid surfaces** and **minimal decorative effects**. Shared token palette with Ragnarok.

## Token palette (default dark)

| Token | Hex | Usage |
|-------|-----|--------|
| Background | `#0A0C11` | App shell |
| Surface | `#11151C` | Cards, panels, navbar |
| Elevated | `#161B24` | Hover, inputs context |
| Border | `#1F2937` | Default borders |
| Border strong | `#374151` | Emphasis |
| Text primary | `#F1F5F9` | Headings |
| Text secondary | `#94A3B8` | Labels |
| Accent | `#3B82F6` | Primary actions (`aether`) |
| AI accent | `#A855F7` | Intelligence features (`aether-ai`) |

CSS variables live in [`src/secure-design.css`](../src/secure-design.css).

## Theme variants

Three variants share the **same solid surfaces**; only accent hues differ:

- **dark** — blue accent (`aether` / `#3B82F6`)
- **steel** — cooler steel-blue borders
- **aurora** — purple/cyan accents on borders and active states

## Component classes

| Class | Purpose |
|-------|---------|
| `.overview-section-shell` | Page header / section wrapper |
| `.dash-card` / `.glass-panel-card` | Content panels |
| `.glass-metric-card` | Stat tiles (solid; legacy name kept) |
| `.tab-chip` | Subnav tabs |
| `.dash-breadcrumb` | Breadcrumb bar |
| `.navbar-solid` | Top navigation |
| `.glass-input` / `.glass-select` | Form inputs |
| `.btn-primary` / `.btn-secondary` / `.btn-danger` | Buttons |

## Rules

- No `backdrop-filter` on dashboard surfaces
- Max radius `rounded-2xl` (16px); prefer `rounded-xl` (12px)
- Shadows: `shadow-sm` / `shadow-md` only on elevated UI (modals, dropdowns)
- Login: solid card on `#0A0C11` background; security footer cue on form

## Files

- Tokens & surfaces: `src/secure-design.css`
- Utilities & animations: `src/index.css`
- Login: `src/zyvor-premium-login.css`, `src/components/PremiumLoginShell.tsx`
- Theme helpers: `src/utils/themeSurface.ts`
- Smoke tests: `tests/secure-design-smoke.spec.ts`
