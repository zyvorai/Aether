// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import { cn } from '../lib/cn';

interface StatCardProps {
  title: string;
  value: string | number;
  color: 'primary' | 'green' | 'red' | 'blue' | 'purple' | 'yellow' | 'orange';
  icon?: ReactNode;
  isEmpty?: boolean;
  compact?: boolean;
}

const accentMap: Record<StatCardProps['color'], string> = {
  primary: 'text-primary',
  orange: 'text-primary',
  green: 'text-emerald-500',
  red: 'text-red-500',
  blue: 'text-sky-500',
  purple: 'text-violet-500',
  yellow: 'text-amber-500',
};

export default function StatCard({ title, value, color, icon, isEmpty, compact }: StatCardProps) {
  const numericEmpty = typeof value === 'number' && value === 0;
  const empty = isEmpty ?? numericEmpty;

  return (
    <div
      className={cn(
        'tahoe-stat-tile relative rounded-[var(--radius-md)] border border-border bg-surface p-4 transition hover:border-primary/30 hover:shadow-card',
        compact && 'p-3',
        empty && 'opacity-70',
      )}
    >
      {icon ? (
        <div className={cn('absolute right-3 top-3 opacity-80', accentMap[color])}>{icon}</div>
      ) : null}
      <p className="mb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-muted">{title}</p>
      <p className={cn('tahoe-stat-value', compact ? 'text-2xl' : 'text-3xl', empty ? 'text-muted' : 'text-foreground')}>
        {value}
      </p>
    </div>
  );
}
