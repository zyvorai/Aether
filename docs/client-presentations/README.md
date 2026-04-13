# Aether Client Presentations

7 slide decks covering business value, architecture, migration, security, quickstart, observability, and developer experience.

All presentations available as HTML (viewable in browser) and can be printed to PDF.

## Presentation Index

### Business & Executive

| # | Title | Pages | Audience |
|---|-------|-------|----------|
| 01 | Business Value | 8 | C-suite, VP Infrastructure, Engineering Directors |
| 05 | Quick Start PoC | 6 | Pre-sales, Solutions Architects |

### Architecture & Technical Deep Dives

| # | Title | Pages | Focus |
|---|-------|-------|-------|
| 02 | Technical Architecture | 8 | Core components, data flow, adapter pattern |
| 07 | Developer Experience | 6 | CLI, templates, compose, watch mode |

### Migration & Operations

| # | Title | Pages | Focus |
|---|-------|-------|-------|
| 03 | Migration Strategies | 8 | Immediate, Blue-Green, Rolling strategies |
| 06 | Observability & Monitoring | 6 | Metrics, health, events, cost estimation |

### Security & Compliance

| # | Title | Pages | Focus |
|---|-------|-------|-------|
| 04 | Security & Compliance | 8 | AES-256-GCM, API auth, audit integrity, policies |

## Generating PDFs

```bash
cd docs/client-presentations/

# Single presentation
google-chrome --headless --print-to-pdf=01-business-value.pdf \
  --print-to-pdf-no-header --no-margins 01-business-value.html

# All presentations
for f in *.html; do
  google-chrome --headless --print-to-pdf="${f%.html}.pdf" \
    --print-to-pdf-no-header --no-margins "$f"
done
```

## Color Theme

| Element | Color | Hex |
|---------|-------|-----|
| Brand accent | Dark Orange | `#d35400` |
| Dark backgrounds | Deep Navy | `#0f111a` |
| Light backgrounds | Off White | `#fafbfc` |
| Success | Green | `#22c55e` |
| Error | Red | `#ef4444` |
| CTA gradient | Orange → Crimson | `#d35400 → #c0392b` |
