// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export default function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="glass-empty-state">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(168,85,247,0.08),transparent_42%)]" />
      <div className="relative mb-4 rounded-2xl border border-slate-700/60 glass-panel-card p-4 text-slate-500 shadow-inner">
        {icon}
      </div>
      <h3 className="relative text-lg font-semibold text-slate-200">{title}</h3>
      {description && (
        <p className="relative mt-2 max-w-md text-sm leading-relaxed text-slate-500">{description}</p>
      )}
      {action && <div className="relative mt-5">{action}</div>}
    </div>
  );
}
