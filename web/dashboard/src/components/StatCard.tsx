// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

interface StatCardProps {
  title: string;
  value: string | number;
  color: 'orange' | 'green' | 'red' | 'blue' | 'purple' | 'yellow';
  icon?: ReactNode;
  isEmpty?: boolean;
  compact?: boolean;
}

const borderMap: Record<StatCardProps['color'], string> = {
  orange: 'border-orange-500/20',
  green: 'border-emerald-500/20',
  red: 'border-red-500/20',
  blue: 'border-blue-500/20',
  purple: 'border-purple-500/20',
  yellow: 'border-amber-500/20',
};

export default function StatCard({ title, value, color, icon, isEmpty, compact }: StatCardProps) {
  const numericEmpty = typeof value === 'number' && value === 0;
  const empty = isEmpty ?? numericEmpty;

  return (
    <div
      className={`glass-metric-card interactive-lift group relative overflow-hidden ${borderMap[color]} ${compact ? 'stat-card-compact p-4' : 'p-5'} ${empty ? 'stat-card-empty glass-metric-empty' : ''}`}
    >
      {icon ? (
        <div className="absolute right-4 top-4 rounded-lg border border-[#1F2937] bg-[#161B24] p-2 text-slate-400 transition-colors group-hover:text-slate-200">
          {icon}
        </div>
      ) : null}
      <p className="relative mb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-400">
        {title}
      </p>
      <p className={`relative font-semibold tracking-tight ${empty ? 'text-slate-500' : 'text-white'} ${compact ? 'stat-value text-2xl' : 'text-3xl'}`}>
        {value}
      </p>
      {!compact ? <div className="premium-divider relative mt-4 opacity-70" /> : null}
    </div>
  );
}
