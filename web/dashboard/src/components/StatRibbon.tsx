// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
  emerald: 'text-emerald-300',
  amber: 'text-amber-200',
  violet: 'text-violet-200',
  sky: 'text-sky-200',
  red: 'text-red-300',
  aether: 'text-brand',
};

const hoverTone: Record<RibbonTone, string> = {
  white: 'hover:bg-white/[0.03]',
  emerald: 'hover:bg-emerald-500/5',
  amber: 'hover:bg-amber-500/5',
  violet: 'hover:bg-violet-500/5',
  sky: 'hover:bg-sky-500/5',
  red: 'hover:bg-red-500/5',
  aether: 'hover:bg-brand/5',
};

interface StatRibbonProps {
  items: RibbonItem[];
  columns?: number;
  testId?: string;
  className?: string;
}

export default function StatRibbon({ items, columns, testId, className }: StatRibbonProps) {
  const cols = columns ?? Math.min(items.length, 6);
  const gridCols =
    cols >= 6 ? 'sm:grid-cols-3 xl:grid-cols-6' :
    cols === 5 ? 'sm:grid-cols-3 xl:grid-cols-5' :
    cols === 4 ? 'sm:grid-cols-2 xl:grid-cols-4' :
    cols === 3 ? 'sm:grid-cols-3' :
    'sm:grid-cols-2';

  return (
    <section
      className={`grid grid-cols-2 gap-px overflow-hidden rounded-2xl border glass-divider ${gridCols} ${className ?? ''}`}
      data-testid={testId}
    >
      {items.map((item) => {
        const tone = item.tone ?? 'white';
        const inner = (
          <>
            <div className="text-[10px] uppercase tracking-wider text-subtle">{item.label}</div>
            <div className={`mt-0.5 text-xl font-semibold tabular-nums ${valueTone[tone]}`}>{item.value}</div>
          </>
        );
        if (item.onClick) {
          return (
            <button
              key={item.label}
              type="button"
              onClick={item.onClick}
              data-testid={item.testId}
              className={`glass-inset-surface px-3 py-3 text-left transition ${hoverTone[tone]}`}
            >
              {inner}
            </button>
          );
        }
        return (
          <div key={item.label} data-testid={item.testId} className="glass-inset-surface px-3 py-3">
            {inner}
          </div>
        );
      })}
    </section>
  );
}
