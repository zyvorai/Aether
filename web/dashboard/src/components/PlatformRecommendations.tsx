// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useMemo, useState } from 'react';
import { AlertCircle, AlertTriangle, Info, X } from 'lucide-react';
import type { PlatformRecommendation } from '../types/api';
import { dismissRecommendation, getDismissedRecommendations } from '../utils/dismissedRecommendations';

function severityIcon(severity: string) {
  switch (severity) {
    case 'critical':
      return <AlertCircle className="shrink-0 text-red-400" size={18} aria-hidden />;
    case 'warn':
      return <AlertTriangle className="shrink-0 text-amber-400" size={18} aria-hidden />;
    default:
      return <Info className="shrink-0 text-sky-400" size={18} aria-hidden />;
  }
}

function severityBorder(severity: string): string {
  switch (severity) {
    case 'critical':
      return 'border-red-500/30 bg-red-500/5';
    case 'warn':
      return 'border-amber-500/30 bg-amber-500/5';
    default:
      return 'border-slate-600/40 bg-slate-800/30';
  }
}

interface PlatformRecommendationsProps {
  items: PlatformRecommendation[];
  loading?: boolean;
}

export default function PlatformRecommendations({ items, loading }: PlatformRecommendationsProps) {
  const [dismissed, setDismissed] = useState(() => getDismissedRecommendations());

  const visible = useMemo(
    () => items.filter((item) => !dismissed.has(item.id)),
    [items, dismissed],
  );

  function handleDismiss(id: string) {
    dismissRecommendation(id);
    setDismissed((prev) => new Set([...prev, id]));
  }

  if (loading) {
    return (
      <div className="glass-panel-card">
        <div className="skeleton h-6 w-48 rounded mb-4" />
        <div className="space-y-3">
          <div className="skeleton h-20 rounded-xl" />
          <div className="skeleton h-20 rounded-xl" />
        </div>
      </div>
    );
  }

  return (
    <div className="glass-panel-card" data-testid="platform-recommendations">
      <h2 className="text-lg font-semibold text-slate-100 mb-1">Setup recommendations</h2>
      <p className="text-sm text-slate-500 mb-4">
        Optional improvements and remediation steps. Dismiss items you have already addressed.
      </p>
      {visible.length === 0 ? (
        <p className="text-sm text-emerald-300/90 rounded-xl border border-emerald-500/20 bg-emerald-500/5 px-4 py-3">
          No open setup items — platform checks look good for your current configuration.
        </p>
      ) : (
        <ul className="space-y-3">
          {visible.map((item) => (
            <li
              key={item.id}
              className={`rounded-xl border px-4 py-3 ${severityBorder(item.severity)}`}
            >
              <div className="flex gap-3">
                {severityIcon(item.severity)}
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="text-sm font-semibold text-slate-100">{item.title}</p>
                    <span className="rounded-md border border-slate-600/50 px-1.5 py-0.5 text-[10px] uppercase tracking-wider text-slate-500">
                      {item.category}
                    </span>
                  </div>
                  {item.detail ? (
                    <p className="mt-1 text-xs text-slate-400 leading-relaxed">{item.detail}</p>
                  ) : null}
                  <p className="mt-2 text-sm text-slate-200 leading-relaxed">{item.action}</p>
                </div>
                <button
                  type="button"
                  data-testid={`platform-rec-dismiss-${item.id}`}
                  onClick={() => handleDismiss(item.id)}
                  className="shrink-0 rounded-lg p-1 text-slate-500 hover:bg-slate-800/80 hover:text-slate-300"
                  aria-label={`Dismiss ${item.title}`}
                >
                  <X size={16} />
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
