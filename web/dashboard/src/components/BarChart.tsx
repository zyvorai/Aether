// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { getBarColor } from '../utils/formatters';

interface BarChartProps {
  label: string;
  percent: number;
  detail?: string;
}

export default function BarChart({ label, percent, detail }: BarChartProps) {
  const clamped = Math.max(0, Math.min(100, percent));

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-sm text-muted">{label}</span>
        <div className="flex items-center gap-2">
          {detail && <span className="text-xs text-subtle">{detail}</span>}
          <span className="text-sm font-medium text-foreground">{clamped.toFixed(1)}%</span>
        </div>
      </div>
      <div className="h-2.5 rounded-full overflow-hidden border glass-divider glass">
        <div
          className={`h-full rounded-full transition-all duration-500 ${getBarColor(clamped)}`}
          style={{ width: `${clamped}%` }}
        />
      </div>
    </div>
  );
}
