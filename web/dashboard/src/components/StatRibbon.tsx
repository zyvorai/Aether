// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

export type RibbonTone = 'white' | 'emerald' | 'amber' | 'violet' | 'sky' | 'red' | 'aether';

export interface RibbonItem {
  label: string;
  value: string | number;
  tone?: RibbonTone;
  onClick?: () => void;
  testId?: string;
}

const valueTone: Record<RibbonTone, string> = {
  white: 'text-foreground',
  emerald: 'text-success',
  amber: 'text-warning',
  violet: 'text-lavender',
  sky: 'text-primary',
  red: 'text-danger',
  aether: 'text-primary',
};

interface StatRibbonProps {
  items: RibbonItem[];
  columns?: number;
  testId?: string;
  className?: string;
}

/** Sparse Apple-style metric row (replaces dense gap-px card strips). */
export default function StatRibbon({ items, testId, className }: StatRibbonProps) {
  return (
    <section
      className={`flex flex-wrap gap-x-10 gap-y-4 ${className ?? ''}`}
      data-testid={testId}
    >
      {items.map((item) => {
        const tone = item.tone ?? 'white';
        const inner = (
          <>
            <div className={`text-2xl font-semibold tabular-nums ${valueTone[tone]}`}>{item.value}</div>
            <div className="mt-1 text-sm text-muted">{item.label}</div>
          </>
        );
        if (item.onClick) {
          return (
            <button
              key={item.label}
              type="button"
              onClick={item.onClick}
              data-testid={item.testId}
              className="text-left transition hover:opacity-80"
            >
              {inner}
            </button>
          );
        }
        return (
          <div key={item.label} data-testid={item.testId} className="text-left">
            {inner}
          </div>
        );
      })}
    </section>
  );
}
