# Aether Dashboard — Aurora / macOS 26 Glass Design System

The dashboard follows **Aurora's Apple.com anatomy** (hero, chapters, hub bands) rendered through a **macOS 26 ("Tahoe") glass surface system**: translucent floating panels (`backdrop-filter: blur(28px) saturate(180%)`) over three blurred ambient color washes, with **Apple blue** accents (not Zyvor marketing orange).

This is the second time this codebase has landed on "real glass" — 11 earlier "liquid glass sweep" commits built it out fully, a September 2026 pass ("Dashboard redesign foundation") flattened it to opaque flat surfaces for a few days, and this pass reactivated it. The flat era's component structure (`AuroraPage`/`PageHero`/`GlassSection`/`SectionHubPage`) is unchanged — only the CSS backing those components (`.glass`, `.tahoe-*`, `.hub-band`, `.hero-swatch`, nav/sidebar chrome) was re-themed. If you're tempted to flatten this again, read `git log --grep="liquid glass"` first.

## Hierarchy

| Layer | Class / component | Use |
|-------|-------------------|-----|
| Page backdrop | `app-shell`, `.ae-ambient` (3 blurred color washes), `--background` | Full-viewport canvas; the ambient washes sit at `z-index: 0` behind a `position: relative; z-index: 1` content wrapper — see `DashboardShell.tsx` |
| Global nav | `GlobalNav` (44px sticky, `var(--glass-blur)`) | All destinations via top links + flyouts |
| Sidebar | `AetherSidebar` — floating inset glass panel (`m-3.5`, `--radius-2xl`), not flush to the window edge | Hubs-only primary nav, collapsible sections, pinned "Ask Zyra" |
| Page hero | `PageHero` / `AuroraPage` (`.tahoe-hero`) | Eyebrow → `DisplayTitle` → lead → actions, now a real glass panel |
| Highlights | `AppleHighlightsRow` + `.apple-chapter-dark` | Glass "Get the highlights." band over an ambient tone glow |
| Chapters | `.apple-chapter`, `SectionHeader` | One idea per section; large vertical air (`--section-gap` / `space-y-12`) |
| Surfaces | `.glass`, `.glass-fill`, `Card` | Translucent blurred panels — `.glass` sets its own `border-radius`; use `.glass-fill` instead when the call site needs a different explicit `rounded-*` (see Gotchas below) |
| Forms | `.login-*`, `Input`, `Button` (pill / 980px) | Login is now glass-over-ambient too (see Auth below) |

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

Login (`LoginGate`) is glass-over-ambient (`.ae-ambient` + `.login-glass`, pill Sign in) — not a dark mesh hero, and no longer the flat "Store paper" exception it was for a few days; it gets the same glass treatment as the rest of the app.

## Themes

`light` | `dark` | `system` via ThemeContext.
Dark chapters (`.apple-chapter-dark`) are now a `--glass-bg-strong` glass panel with a tone glow in both themes (no more solid-black override) — the page's `--background` still goes true OLED black under `.dark-theme`, so glass panels read as lighter floating surfaces against it, same as the reference macOS 26 look.
**iPad Pro–style dark mode**: `.dark-theme` goes true OLED black (`--background: #000000`) with vivid single-tone glows (`--tone-*-glow`, ~0.32-0.34 opacity) — `.apple-editorial-hero` (every `PageHero`) and `.login-chapter-hero` both get a radial spotlight glow behind the hero copy in dark mode only. Tone comes from `accent`/`data-tone`, defaulting to sky blue.

## Buttons

- Primary: pill (`--radius-pill` / ~980px), 44px min height, blue fill
- Secondary: pill outline blue, tint wash on hover
- Tertiary: text + tint for toolbars
- Legacy `.btn-primary` / `.btn-secondary` mirror `Button` variants

## Conventions

- Prefer `AuroraPage` + sparse chapters, now rendered as glass — don't hand-roll a one-off blur effect when `Card`/`GlassSection`/`.glass`/`.glass-fill` already gets you there
- Pass `stats` (and optional `statsTestId`) to `AuroraPage` / `PageHero` for headline metrics
- Overview ships hero + ink highlights + briefing + next actions + a "One spec, every runtime" feature/orbit card (`.feature-orbit-card`, `RuntimeOrbitFeature` in `OverviewPage.tsx`); Explore disclosure holds secondary inventory
- Nav: `data-testid="global-nav"`
- Hex surfaces stay in CSS/`theme.css` only — `npm run check:hex-surfaces` fails on `bg-[#…]` / `bg-black` in TSX
- Semantic warning colors (amber/orange for degraded health, log WARN) are status-only—not brand
- Runtime identity: `--rt-podman` / `--rt-k8s` / `--rt-kubevirt` alias `--tone-sky` / `--tone-violet` / `--tone-teal` — one source of truth for "Podman is sky, Kubernetes is violet, KubeVirt is teal" everywhere a runtime chip/node renders (`formatters.ts`, hero swatches, the orbit card). There is no fourth runtime — Metal3 was removed from the whole product; don't reintroduce a `--rt-metal3` token or a fourth swatch/node

### Gotchas

- **`.glass` vs `.glass-fill`** — `.glass` sets its own `border-radius: var(--radius-liquid)`, and because it's declared after Tailwind's utility layer in `index.css`, it wins the cascade over an explicit `rounded-*` class on the same element. If you need a pill (`rounded-full`) or a specific radius token (`rounded-[var(--radius-md)]`) on a glass surface, use `.glass-fill` (background + blur + shadow, no radius opinion) instead of `.glass`, and let your own `rounded-*` class win.
- **`@supports` fallback** — every glass surface is wrapped in `@supports (backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))`; browsers without `backdrop-filter` support fall back to an opaque `--surface-elevated` fill instead of a translucent-but-unblurred (illegible) panel. Keep this pattern when adding new glass surfaces.
- **Hub pages are not bespoke mocks** — `FabricPage`, `MigrationsPage`, and `SettingsPage` are `SectionHubPage` indexes (6, 4, and 6 bands respectively) into real sub-features, not the flat "3 runtime cards" / "migration row list" / "toggle rows" shown in early macOS-26 mockups. They inherit the glass look automatically via `SectionHubPage`'s `.hub-band` styling — resist the urge to replace real hub navigation with a simplified mock just because a reference design showed one.

## Verification

```bash
cd web/dashboard && npm run check:hex-surfaces && npm run build
npm run test
npm run test:e2e -- tests/aurora-shell-smoke.spec.ts
```
