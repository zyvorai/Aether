#!/usr/bin/env python3
"""Migrate feature panels from surface-panel sections to GlassSection."""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "src" / "components"

PURPLE = {
    "IntentStudioPanel.tsx",
    "WorkloadDesignerPanel.tsx",
    "RuntimeAdvisorPanel.tsx",
    "SecurityCopilotPanel.tsx",
    "GitOpsAgentPanel.tsx",
    "KnowledgeGraphPanel.tsx",
    "IntentPipelinePanel.tsx",
    "IntentPlatformPanel.tsx",
    "IntentGitOpsDiffPanel.tsx",
    "AutonomousModePanel.tsx",
    "AutonomousSrePanel.tsx",
    "AutonomousPlacementPanel.tsx",
}

NEUTRAL = {"RuntimeFabricGraph.tsx"}

HEADER_RE = re.compile(
    r"""
    <section\s+className="surface-panel(?:\s+mb-8)?\s+rounded-\[28px\]\s+p-6\s+sm:p-8"\s+data-testid="(?P<testid>[^"]+)">\s*
    <div\s+className="mb-(?:4|6)\s+flex(?:\s+flex-wrap)?\s+items-center\s+justify-between\s+gap-3">\s*
      <div\s+className="flex\s+items-center\s+gap-3">\s*
        (?P<icon><[A-Za-z][^>]*className="h-5\s+w-5[^"]*"[^>]*/>)\s*
        <div>\s*
          <h2\s+className="text-xl\s+font-semibold\s+text-white">(?P<title>[^<]+)</h2>\s*
          <p\s+className="text-sm\s+text-slate-400">\s*(?P<subtitle>[\s\S]*?)\s*</p>\s*
        </div>\s*
      </div>\s*
      (?:(?P<actions><div\s+className="flex[^"]*">[\s\S]*?</div>)\s*)?
    </div>\s*
    """,
    re.VERBOSE,
)

INNER_DIV_RE = re.compile(
    r"""
    <div\s+className="surface-panel\s+rounded-\[28px\]\s+p-6\s+sm:p-8">\s*
    <div\s+className="mb-6\s+flex\s+items-center\s+gap-3">\s*
      (?P<icon><[A-Za-z][^>]*className="h-5\s+w-5[^"]*"[^>]*/>)\s*
      <div>\s*
        <h2\s+className="text-xl\s+font-semibold\s+text-white">(?P<title>[^<]+)</h2>\s*
        <p\s+className="text-sm\s+text-slate-400">\s*(?P<subtitle>[\s\S]*?)\s*</p>\s*
      </div>\s*
    </div>\s*
    """,
    re.VERBOSE,
)


def accent_for(name: str) -> str:
    if name in NEUTRAL:
        return "neutral"
    if name in PURPLE:
        return "purple"
    return "blue"


def ensure_import(content: str) -> str:
    if "GlassSection" in content:
        return content
    lines = content.split("\n")
    insert_at = 0
    for i, line in enumerate(lines):
        if line.startswith("import "):
            insert_at = i + 1
    lines.insert(insert_at, "import GlassSection from './GlassSection';")
    return "\n".join(lines)


def escape_js(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"').strip()


def build_glass_section(
    testid: str | None,
    title: str,
    subtitle: str,
    icon: str,
    actions: str | None,
    accent: str,
) -> str:
    parts = [
        f'    <GlassSection',
        f'      accent="{accent}"',
    ]
    if testid:
        parts.append(f'      testId="{testid}"')
    parts.append(f'      title="{escape_js(title)}"')
    parts.append(f'      subtitle="{escape_js(subtitle)}"')
    parts.append(f"      icon={{{icon}}}")
    if actions:
        parts.append(f"      actions={{{actions}}}")
    parts.append("    >")
    return "\n".join(parts)


def migrate_section_panel(content: str, filename: str) -> str:
    accent = accent_for(filename)

    def repl(m: re.Match) -> str:
        actions = m.group("actions")
        return build_glass_section(
            m.group("testid"),
            m.group("title"),
            m.group("subtitle"),
            m.group("icon"),
            actions,
            accent,
        )

    new_content, n = HEADER_RE.subn(repl, content, count=1)
    if n:
        new_content = new_content.replace("</section>", "</GlassSection>", 1)
        return ensure_import(new_content)
    return content


def migrate_inner_div_panel(content: str, filename: str) -> str:
    accent = accent_for(filename)

    def repl(m: re.Match) -> str:
        return build_glass_section(
            None,
            m.group("title"),
            m.group("subtitle"),
            m.group("icon"),
            None,
            accent,
        )

    new_content, n = INNER_DIV_RE.subn(repl, content, count=1)
    if n:
        # Replace closing div for surface-panel - find matching close before section end
        # The inner div wraps content; replace first standalone </div> at end of component carefully
        # Pattern: content ends with </div>\n    </section> or similar
        new_content = re.sub(
            r"(</GlassSection>\s*\n\s*)(\s*</div>)",
            r"\1",
            new_content,
            count=0,
        )
        # Remove extra closing div that was for surface-panel wrapper
        idx = new_content.find("<GlassSection")
        if idx >= 0:
            close_idx = new_content.rfind("</div>")
            glass_close = new_content.rfind("</GlassSection>")
            if close_idx > glass_close and glass_close >= 0:
                # remove the div close right after GlassSection close
                after = new_content[glass_close + len("</GlassSection>") :]
                after = re.sub(r"^\s*</div>", "", after, count=1)
                new_content = new_content[: glass_close + len("</GlassSection>")] + after
        return ensure_import(new_content)
    return content


def main() -> int:
    changed = 0
    for path in sorted(ROOT.glob("*Panel.tsx")):
        content = path.read_text()
        if "GlassSection" in content and "surface-panel rounded-[28px]" not in content:
            continue
        original = content
        content = migrate_section_panel(content, path.name)
        if content == original:
            content = migrate_inner_div_panel(content, path.name)
        if content != original:
            path.write_text(content)
            print(f"migrated {path.name}")
            changed += 1
        else:
            if "surface-panel rounded-[28px]" in original:
                print(f"SKIP (manual): {path.name}", file=sys.stderr)

    graph = ROOT / "RuntimeFabricGraph.tsx"
    if graph.exists():
        content = graph.read_text()
        if "surface-panel" in content and "GlassSection" not in content:
            print("SKIP (manual): RuntimeFabricGraph.tsx", file=sys.stderr)

    print(f"Done: {changed} files migrated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
