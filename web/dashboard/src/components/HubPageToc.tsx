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
      className="mb-10 flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-baseline sm:gap-x-8 sm:gap-y-2"
    >
      <span className="text-xs font-medium uppercase tracking-wider text-subtle shrink-0">On this page</span>
      {items.map((item) => (
        <a
          key={item.id}
          href={`#${item.id}`}
          className="text-sm text-primary hover:underline"
        >
          {item.label}
        </a>
      ))}
    </nav>
  );
}
