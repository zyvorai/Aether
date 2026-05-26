// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
        <span className="text-sm text-zinc-300">{label}</span>
        <div className="flex items-center gap-2">
          {detail && <span className="text-xs text-zinc-500">{detail}</span>}
          <span className="text-sm font-medium text-zinc-200">{clamped.toFixed(1)}%</span>
        </div>
      </div>
      <div className="h-2.5 bg-zinc-700/50 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-500 ${getBarColor(clamped)}`}
          style={{ width: `${clamped}%` }}
        />
      </div>
    </div>
  );
}
