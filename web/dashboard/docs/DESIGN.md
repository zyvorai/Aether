# Aether Dashboard — Aurora / Apple Design System

The dashboard follows **Aurora’s Apple.com anatomy** with **Apple blue** accents from Aurora `globals.css` (not Zyvor marketing orange).

## Hierarchy

| Layer | Class / component | Use |
|-------|-------------------|-----|
| Page backdrop | `app-shell`, `--background` | Full-viewport canvas (light-first paper `#f5f5f7`) |
| Global nav | `GlobalNav` (44px sticky blur) | All destinations via top links + flyouts |
| Page hero | `PageHero` / `AuroraPage` | Eyebrow → `DisplayTitle` → lead → actions |
| Highlights | `AppleHighlightsRow` + `.apple-chapter-dark` | Ink “Get the highlights.” band (iPad Pro chapter) |
| Chapters | `.apple-chapter`, `SectionHeader` | One idea per section; large vertical air (`--section-gap` / `space-y-12`) |
| Surfaces | `.glass`, `Card` | Flat Apple cards only when interaction needs a container |
| Forms | `.login-*`, `Input`, `Button` (pill / 980px) | Apple Store paper login + console controls |

## Apple.com → Aether page map

Three named apple.com pages are the concrete templates — matched by page role, not applied uniformly:

| Template | Role | Aether pages |
|---|---|---|
| **apple.com/apple-vision-pro** — colors & type | Dashboard-wide typography/color system: tight tracking (`--tracking-display` / `--tracking-hero`), uppercase eyebrow chapter-dividers, the 7-tone depth system (`--tone-*` in `theme.css`) replacing flat single-blue | Every page (global refresh via `Typography.tsx`, `theme.css`, `index.css`) |
| **apple.com/ipad** — flagship hero | Bold hero + an iPad-style color-swatch picker/legend row (`PageHero` `swatches` prop, `.hero-swatch-row`) | Overview, Login (`LoginGate`) |
| **apple.com/services** — hub bands | One full-width, tone-tinted band per destination (`SectionHubPage`, `.hub-band`) instead of a plain divide-y list — each hub child gets its own accent like each Apple service does | All 18 `SectionHubPage` hubs: Observability, Settings, Labs, Migrations, Fabric, AI, Intelligence, Fleet, Confidential, Platform, Metrics, Security, GitOps, Cost, Activity, Affinity, Alerts, Clusters |
| “Get the highlights.” ink band | 3–5 large metrics, now with a per-page tone glow (`accent` → `data-tone` on the band) | Via `AuroraPage` / `PageHero` `stats` |
| Store browse | Sparse list or large quiet rows — deliberately **not** given hub-band or hero treatment; dense pages stay restrained | Workloads, Applications, Clusters, Helm, Templates, Plugins |
| Support / account forms | Paper, single column, generous gaps | Secrets, RBAC, Alerts, Editor, Compose, Settings children |
| Spec / comparison tables | Clean rows, no card nests | Events, Audit, Metrics, Activity, Drift, GitOps |

## Density rules

- **One idea per chapter** — do not stack briefing + metrics + chip walls + tables in the first viewport
- **Hub pages are indexes** — Observability, Settings, Labs, Migrations, Fabric, AI, Intelligence, Fleet, Confidential, Platform, Metrics, Security, GitOps, Cost, Activity, Affinity, Alerts, and Clusters render as Services-style tone-banded rows (`SectionHubPage`), capped at ~6 bands; panels live on child routes (`sidebar: false`) with breadcrumbs back to the hub
- **Sidebar is hubs-only** — primary Overview / Workloads / Fabric; Intelligence (Intelligence + Confidential); Operate (Observability, Fleet, Clusters, Migrations); Platform (Labs, Settings). Cap each hub index at ~6 chapter rows. AI, Zyra, and Applications are hub children / Cmd+K — not peer rail items.
- **Quiet shell** — no floating Ask Aether / agent docks; Zyra rail only on Zyra/copilot views
- **Workload detail** — three tabs (Overview / Logs / Spec) plus ≤4 Next actions; no Related-pages chip walls
- **No stacked stat strips** — headline numbers go only through ink `stats` (or a sparse text metric row), never a 4-up `StatCard` grid plus highlights
- Prefer list rows / tables over card grids; remove nested glass-in-glass
- Collapse secondary navigation (quick-link chip walls) into GlobalNav / command palette or a single Explore disclosure (≤8 links)
- Preserve all `data-testid` attributes when restyling

## Brand

- Primary: Apple blue `#0071e3` (light) / `#0a84ff` (dark)
- Neutrals: Apple `#ffffff` / `#f5f5f7` / `#1d1d1f`
- Aliases: `--apple-ink` / `--apple-paper` / `--apple-action` / `--font-hero` in `theme.css`
- **Tone system** (Vision Pro–derived depth, not decorative): `--tone-{sky,violet,emerald,amber,pink,teal,rust}` each ship a solid, `-tint` wash, and `-glow` radial variant; bind with `data-tone="…"` on a wrapper (see `[data-tone]` rules in `index.css`) and consume via `var(--tone-color)` / `var(--tone-wash)` / `var(--tone-glow)`. Same `Tone` union as `PageHero`'s `accent`/`swatches`/`stats[].tone` and `SectionHubPage`'s `HubLink.tone`
- Tracking: `--tracking-eyebrow: 0.08em` (uppercase chapter dividers), `--tracking-display: -0.035em` (section/page titles), `--tracking-hero: -0.045em` (hero display type)
- Display: Archivo / SF Pro; body SF / system
- Layout spacing: zyvor-web nav anatomy (44px bar, 1180px max width); chapter rhythm `--section-gap: 3.5rem`

## Auth (demo)

Default local login (disable with `AETHER_DEMO_AUTH=0`):

- Username: `admin`
- Password: `Admin@321`

Override: `AETHER_DEMO_USER` / `AETHER_DEMO_PASSWORD`.

Login (`LoginGate`) is **Apple Store paper** (white/light chapters + pill Sign in) — not a dark mesh hero.

## Themes

`light` | `dark` | `system` via ThemeContext.
Dark chapters (`.apple-chapter-dark`) use ink `#1d1d1f` in light theme and pure black under `.dark-theme`.
**iPad Pro–style dark mode**: `.dark-theme` goes true OLED black (`--background: #000000`) with vivid single-tone glows (`--tone-*-glow`, ~0.32-0.34 opacity) — `.apple-editorial-hero` (every `PageHero`) and `.login-chapter-hero` both get a radial spotlight glow behind the hero copy in dark mode only (light mode stays plain Store paper, no glow). Tone comes from `accent`/`data-tone`, defaulting to sky blue.

## Buttons

- Primary: pill (`--radius-pill` / ~980px), 44px min height, blue fill
- Secondary: pill outline blue, tint wash on hover
- Tertiary: text + tint for toolbars
- Legacy `.btn-primary` / `.btn-secondary` mirror `Button` variants

## Conventions

- Prefer `AuroraPage` + sparse chapters over Liquid Glass shells
- Pass `stats` (and optional `statsTestId`) to `AuroraPage` / `PageHero` for headline metrics
- Overview ships hero + ink highlights + two quiet chapters (briefing, next actions); Explore disclosure holds secondary inventory
- Nav: `data-testid="global-nav"`
- Hex surfaces stay in CSS/`theme.css` only — `npm run check:hex-surfaces` fails on `bg-[#…]` / `bg-black` in TSX
- Semantic warning colors (amber/orange for degraded health, log WARN) are status-only—not brand

## Verification

```bash
cd web/dashboard && npm run check:hex-surfaces && npm run build
npm run test
npm run test:e2e -- tests/aurora-shell-smoke.spec.ts
```
