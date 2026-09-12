# Copyright 2026 ZyvorAI Labs Private Limited
# SPDX-License-Identifier: Apache-2.0
"""MkDocs build hook.

Renders the Apple-glass hero / highlights / hub-band Jinja macros
(docs/overrides/partials/*.html) from a page's YAML front matter, and
prepends the resulting HTML to the page's markdown. `md_in_html` (already
enabled in mkdocs.yml) passes the raw HTML through untouched.

A page opts in with front matter like:

    ---
    hero:
      eyebrow: "PRODUCT"
      title: "Universal Runtime Portability"
      lead: "Deploy once. Move workloads across containers, Kubernetes, and VMs."
      tone: violet
      swatches:
        - {label: Podman, tone: sky}
      highlights:
        - {value: "3", label: "Runtimes, one spec"}
      hub_bands:
        - {title: Compose, description: "...", href: features/compose.md, tone: sky}
    ---

Keeping markdown as data (front matter), not markup, means one template
change updates every hero page at once, and lets the macros mechanically
enforce the density caps (<=5 highlights, <=6 hub bands) in one place.
"""

from pathlib import Path

import jinja2

PARTIALS_DIR = Path(__file__).parent / "partials"
_env = jinja2.Environment(loader=jinja2.FileSystemLoader(str(PARTIALS_DIR)), autoescape=False)


def on_page_markdown(markdown, page, config, files):
    hero_data = page.meta.get("hero")
    if not hero_data:
        return markdown

    tone = hero_data.get("tone", "sky")

    highlights_html = ""
    items = hero_data.get("highlights")
    if items:
        highlights_html = _env.get_template("highlights.html").module.highlights(items, tone=tone)

    hero_html = _env.get_template("hero.html").module.hero(
        eyebrow=hero_data.get("eyebrow", ""),
        title=hero_data.get("title") or page.title or "",
        lead=hero_data.get("lead", ""),
        tone=tone,
        swatches=hero_data.get("swatches", []),
        actions=hero_data.get("actions", []),
        highlights_html=highlights_html,
    )

    hub_html = ""
    bands = hero_data.get("hub_bands")
    if bands:
        hub_html = _env.get_template("hub_band.html").module.hub_band(bands)

    return f"{hero_html}\n\n{hub_html}\n\n{markdown}"
