// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

export type AppleHighlight = {
  id: string;
  value: string;
  label: string;
};

export default function AppleHighlightsRow({
  title,
  items,
  className = '',
}: {
  title?: string;
  items: AppleHighlight[];
  className?: string;
}) {
  if (!items.length) return null;
  return (
    <div className={`apple-highlights-row ${className}`.trim()} data-testid="apple-highlights-row">
      {title ? <p className="apple-highlights-kicker">{title}</p> : null}
      <ul className="apple-highlights-grid list-none m-0 p-0">
        {items.map((item) => (
          <li key={item.id} className="apple-highlight-item">
            <div className="apple-highlight-value">{item.value}</div>
            <div className="apple-highlight-label">{item.label}</div>
          </li>
        ))}
      </ul>
    </div>
  );
}
