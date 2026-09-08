// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
  green: 'text-success',
  red: 'text-danger',
  blue: 'text-primary',
  purple: 'text-lavender',
  yellow: 'text-warning',
};

export default function StatCard({ title, value, color, icon, isEmpty, compact }: StatCardProps) {
  const numericEmpty = typeof value === 'number' && value === 0;
  const empty = isEmpty ?? numericEmpty;

  return (
    <div
      className={cn(
        'tahoe-stat-tile glass-hover-lift relative rounded-[var(--radius-md)] p-4',
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
