// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

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

const ACCENT_TONE: Record<MetricAccent, string> = {
  blue: 'sky',
  purple: 'violet',
  emerald: 'emerald',
  amber: 'amber',
  muted: 'slate',
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
      className="tahoe-stat-tile glass-hover-lift group w-full p-4 text-left"
      data-testid={testId}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="mb-2.5 flex items-center gap-2 text-[13px] font-medium text-muted">
            <span className="metric-chip" data-tone={ACCENT_TONE[accent]} aria-hidden>{icon}</span>
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
