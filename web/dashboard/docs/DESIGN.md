# Aether Dashboard Design System

A quiet, neutral interface: large tight headlines, generous space, flat cards, one blue accent, and light and dark themes that are equals.

## Tokens

Every colour is a CSS variable in `src/theme.css`. `src/index.css` maps them onto Tailwind utilities with `@theme inline`; there is no `tailwind.config.js`.

| Role | Token / utility | Light | Dark |
|------|-----------------|-------|------|
| Page | `bg-background` | `#f5f5f7` | `#000000` |
| Card | `bg-surface-elevated`, `.glass` | `#ffffff` | `#1d1d1f` |
| Inset fill | `bg-surface` | `#f2f2f4` | `#2a2a2c` |
| Text | `text-foreground` / `text-muted` / `text-subtle` | `#1d1d1f` / `#515154` / `#68686d` | `#f5f5f7` / `#b4b4b9` / `#9d9da2` |
| Accent text, icons, tints | `text-primary`, `bg-primary/10` | `#005cb9` | `#409cff` |
| Accent fill under white text | `bg-accent-fill` | `#0071e3` | `#0071e3` |
| Status text | `text-success` / `text-warning` / `text-danger` | AA on every surface | AA on every surface |
| Status fill under white text | `bg-success-fill` / `bg-warning-fill` / `bg-danger-fill` | | |

Rules:

- Solid fills that carry white text use the `*-fill` tokens, never `bg-primary` or `bg-danger`.
- Tints behind coloured text stay at `/10` so the text keeps 4.5:1.
- Do not fade text with `opacity-*`; use `text-muted` or `text-subtle`.
- No hex or black surface fills in TSX (`npm run check:hex-surfaces`).

## Themes

`light`, `dark` and `system` (default), chosen from the Appearance menu (`ThemeToggle`, in the navigation bar and on the sign-in page) and stored in `localStorage['aether-theme']`.

The inline script in `index.html` resolves the theme and sets `<html data-theme="light|dark">` (plus `light-theme` / `dark-theme`) before first paint; `ThemeContext` keeps it in sync, including live operating-system changes. Style dark variants with `:root[data-theme='dark']`, normally by overriding a token only.

## Type

System fonts only (`-apple-system`, SF Pro, then Inter / Segoe UI / system-ui). No webfont downloads. Body is 17px; page titles are `.page-title` (26px, 22px on phones) so that data starts on the first screen; `text-display` is for the sign-in page; mono is SF Mono / `ui-monospace`.

## Shape and layout

- Cards: 18px radius, hairline edge, soft shadow. A card inside a card becomes an inset fill.
- Controls: pill buttons (44px primary on touch, 36px with a fine pointer; `.btn-quiet` for secondary toolbar actions), 12px inputs.
- `rounded-sm` to `rounded-2xl` come from the `@theme` radius scale.
- Navigation bar: 52px, sticky, translucent and blurred. Dialogs and menus use the same material.
- Hub pages are a grid of one card per destination (`.hub-band`), coloured only by the icon (`data-tone`).
- Headline metrics use `StatRibbon` (clickable stats double as filters); do not stack it with `PageHero` `stats`.
- Vertical rhythm is `.page-stack` and `--stack-gap`; do not add larger `space-y-*` gaps between page sections.

## Console shell (`src/shell.css`)

| Piece | Where | Rules |
|-------|-------|-------|
| Information architecture | `utils/navSections.ts` | Sidebar, breadcrumb and section tabs derive from `DASHBOARD_VIEWS`. A path under another page's path is its child; an unlisted top-level page falls back to a section by `group`. List a view in `NAV_SECTIONS` only to place or order it better. |
| Sidebar | `AetherSidebar` | All sections, pinned pages first, filter over every page, icon rail (`[`); a drawer below 1024px. |
| Top bar | `layout/GlobalNav` | Search, data freshness with pause, unacknowledged events, help, appearance, account. |
| Page header | `AuroraPage` | Breadcrumb with pin star, title, one-line description, tabs for hubs. |
| Tables | `ui/DataTable` | Table roles, `aria-sort`, `j`/`k`/arrows, Enter opens, 100 rows per step, cards below 640px, compact density. |
| Record detail | `ui/Drawer` | Side panel with previous/next, Escape, focus returned to the opener. |
| States | `PageLoading`, `PageLoadError`, `EmptyState` | Keep the page header. A failed request is an error with the failing call, never an empty list or a zero. |
| Preferences | `utils/preferences.ts` | Pins, density, sidebar and the orientation card live in this browser only. |

Keyboard shortcuts are listed in the Help dialog (`?`); `g` sequences come from `utils/goShortcuts.ts`.

## Accessibility

- Focus is always visible: a 2px ring on `:focus-visible`, unlayered so utilities cannot remove it.
- Icon-only controls need an `aria-label`; form controls need a label.
- Motion is disabled under `prefers-reduced-motion`.

## Cascade layers (Tailwind 4)

Utilities live in `@layer utilities`. Any unlayered rule beats them, including a global `* { margin: 0 }` reset. Put component classes in `@layer components`, and keep element resets out of `index.html`.

## Data conventions

- Runtime identity: `--rt-podman` / `--rt-k8s` / `--rt-kubevirt` alias `--tone-sky` / `--tone-violet` / `--tone-teal`. There is no fourth runtime.
- Status colours are for status only.
- `workload.name` is not the pod name for Kubernetes and KubeVirt; `WorkloadDetail.tsx` resolves pod names via `/cluster/resource`.
- Preserve `data-testid` attributes when restyling.

## Auth (demo)

Default local login (disable with `AETHER_DEMO_AUTH=0`): `admin` / `Admin@321`. Override with `AETHER_DEMO_USER` / `AETHER_DEMO_PASSWORD`.

## Verification

```bash
cd web/dashboard && npm run check:hex-surfaces && npm run build
npm run test
npm run test:e2e -- tests/aurora-shell-smoke.spec.ts
```
