// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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

const accentClass: Record<MetricAccent, string> = {
  blue: 'glass-metric-accent-blue',
  purple: 'glass-metric-accent-purple',
  emerald: 'glass-metric-accent-emerald',
  amber: 'glass-metric-accent-amber',
  muted: 'glass-metric-accent-muted',
};

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
      className={`glass-metric-card group text-left ${accentClass[accent]} ${isEmpty ? 'glass-metric-empty' : ''}`}
      data-testid={testId}
    >
      <div className="relative flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="mb-3 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">
            {icon}
            {label}
          </div>
          <div
            className={`text-3xl font-semibold tracking-tight ${isEmpty ? 'text-slate-500' : 'text-white'}`}
            data-testid={valueTestId}
          >
            {value}
          </div>
          {hint ? (
            <p className="mt-2 text-xs text-slate-500 transition group-hover:text-slate-400">{hint}</p>
          ) : null}
        </div>
        <div className="glass-metric-icon-wrap">{icon}</div>
      </div>
      {children}
    </button>
  );
}
