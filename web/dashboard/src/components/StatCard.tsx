// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

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
        'tahoe-stat-tile relative p-4',
        compact && 'p-3',
      )}
    >
      {icon ? (
        <div className={cn('absolute right-3.5 top-3.5', empty ? 'text-subtle' : accentMap[color])}>{icon}</div>
      ) : null}
      <p className={cn('mb-1.5 text-[0.8125rem] font-medium text-muted', icon ? 'pr-8' : '')}>{title}</p>
      <p className={cn('tahoe-stat-value', compact ? 'text-2xl' : 'text-3xl', empty ? 'text-muted' : 'text-foreground')}>
        {value}
      </p>
    </div>
  );
}
