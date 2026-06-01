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

const colorMap: Record<StatCardProps['color'], string> = {
  orange: 'stat-card-orange',
  green: 'stat-card-green',
  red: 'stat-card-red',
  blue: 'stat-card-blue',
  purple: 'stat-card-purple',
  yellow: 'stat-card-yellow',
};

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
      className={`${colorMap[color]} ${borderMap[color]} glass-metric-card interactive-lift group relative overflow-hidden transition-all duration-200 ${compact ? 'stat-card-compact p-4' : 'p-5'} ${empty ? 'stat-card-empty' : ''}`}
    >
      <div className="pointer-events-none absolute inset-0 bg-gradient-to-br from-white/[0.08] via-transparent to-transparent opacity-70" />
      <div className="pointer-events-none absolute -right-8 -top-8 h-24 w-24 rounded-full bg-white/10 blur-2xl transition-opacity group-hover:opacity-100" />
      {icon && (
        <div className="absolute right-4 top-4 rounded-xl border border-white/10 bg-white/[0.04] p-2 text-slate-300/70 transition-colors group-hover:text-white">
          {icon}
        </div>
      )}
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
