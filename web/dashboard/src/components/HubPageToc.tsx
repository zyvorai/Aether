// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

export interface HubTocItem {
  id: string;
  label: string;
}

interface HubPageTocProps {
  items: HubTocItem[];
}

export default function HubPageToc({ items }: HubPageTocProps) {
  if (items.length < 3) return null;

  return (
    <nav
      aria-label="On this page"
      className="glass-toolbar sticky top-0 z-10 flex flex-wrap gap-2"
    >
      <span className="text-xs font-medium uppercase tracking-wider text-subtle">On this page</span>
      {items.map((item) => (
        <a
          key={item.id}
          href={`#${item.id}`}
          className="quick-link-chip text-xs"
        >
          {item.label}
        </a>
      ))}
    </nav>
  );
}
