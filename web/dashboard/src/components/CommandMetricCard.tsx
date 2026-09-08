// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';

type MetricAccent = 'blue' | 'purple' | 'emerald' | 'amber' | 'muted';

interface CommandMetricCardProps {
  label: string;
  value: ReactNode;
  icon: ReactNode;
  accent: MetricAccent;
  hint?: string;
  isEmpty?: boolean;
  onClick?: () => void;
  testId?: string;
  valueTestId?: string;
  children?: ReactNode;
}

export default function CommandMetricCard({
  label,
  value,
  icon,
  accent,
  hint,
  isEmpty = false,
  onClick,
  testId,
  valueTestId,
  children,
}: CommandMetricCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`tahoe-stat-tile group w-full rounded-[var(--radius-md)] border border-border bg-surface p-4 text-left transition hover:border-primary/30 hover:shadow-card ${isEmpty ? 'opacity-70' : ''}`}
      data-testid={testId}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-muted">
            {icon}
            {label}
          </div>
          <div
            className={`tahoe-stat-value text-2xl ${isEmpty ? 'text-muted' : 'text-foreground'}`}
            data-testid={valueTestId}
          >
            {value}
          </div>
          {hint ? <p className="mt-2 text-xs text-muted group-hover:text-foreground">{hint}</p> : null}
        </div>
      </div>
      {children}
    </button>
  );
}
