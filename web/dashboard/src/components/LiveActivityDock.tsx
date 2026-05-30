// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Rocket, X } from 'lucide-react';

export interface LiveActivity {
  id: string;
  kind: 'migration';
  title: string;
  subtitle: string;
  percent: number;
  etaSecs: number | null;
  phase: string;
  createdAt: number;
}

function formatEta(secs: number | null): string {
  if (secs === null || secs <= 0) return 'Done';
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}m ${s}s`;
}

export default function LiveActivityDock() {
  const [activities, setActivities] = useState<LiveActivity[]>([]);

  const upsert = useCallback((next: Omit<LiveActivity, 'createdAt'> & { createdAt?: number }) => {
    setActivities((prev) => {
      const existing = prev.find((a) => a.id === next.id);
      const item: LiveActivity = {
        ...next,
        createdAt: next.createdAt ?? existing?.createdAt ?? Date.now(),
      };
      const filtered = prev.filter((a) => a.id !== next.id);
      if (item.phase === 'completed' || item.phase === 'failed') {
        setTimeout(() => {
          setActivities((p) => p.filter((a) => a.id !== item.id));
        }, item.phase === 'completed' ? 8000 : 12000);
      }
      return [item, ...filtered].slice(0, 4);
    });
  }, []);

  useEffect(() => {
    const onActivity = (event: Event) => {
      const detail = (event as CustomEvent<{
        workload?: string;
        phase?: string;
        percent?: number;
        eta_secs?: number | null;
        etaSecs?: number | null;
        message?: string;
      }>).detail;
      if (!detail?.workload) return;
      upsert({
        id: `migration-${detail.workload}`,
        kind: 'migration',
        title: detail.phase === 'completed' ? 'Migration complete' : 'Migration running',
        subtitle: detail.message ?? detail.workload,
        percent: detail.percent ?? 0,
        etaSecs: detail.eta_secs ?? detail.etaSecs ?? null,
        phase: detail.phase ?? 'running',
      });
    };
    window.addEventListener('aether-live-activity', onActivity);
    return () => window.removeEventListener('aether-live-activity', onActivity);
  }, [upsert]);

  if (activities.length === 0) return null;

  return (
    <div
      className="fixed bottom-6 left-6 z-40 flex max-w-sm flex-col gap-2"
      data-testid="live-activity-dock"
    >
      {activities.map((activity) => (
        <div
          key={activity.id}
          className="animate-fade-in overflow-hidden rounded-2xl border border-slate-700/60 bg-slate-950/90 p-4 shadow-2xl backdrop-blur-xl"
          data-testid={`live-activity-${activity.kind}`}
        >
          <div className="mb-2 flex items-start justify-between gap-2">
            <div className="flex items-center gap-2">
              <Rocket className={`h-4 w-4 ${activity.phase === 'failed' ? 'text-red-400' : 'text-aether'}`} />
              <div>
                <div className="text-sm font-semibold text-white">{activity.title}</div>
                <div className="text-xs text-slate-500 truncate max-w-[220px]">{activity.subtitle}</div>
              </div>
            </div>
            <button
              type="button"
              onClick={() => setActivities((p) => p.filter((a) => a.id !== activity.id))}
              className="text-slate-500 hover:text-slate-300"
              aria-label="Dismiss"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
          <div className="mb-1 flex items-center justify-between text-[10px] uppercase tracking-wider text-slate-500">
            <span>{activity.percent}%</span>
            <span>ETA {formatEta(activity.etaSecs)}</span>
          </div>
          <div className="h-1.5 overflow-hidden rounded-full bg-slate-800">
            <div
              className={`h-full rounded-full transition-all duration-500 ${
                activity.phase === 'failed'
                  ? 'bg-red-500'
                  : activity.phase === 'completed'
                    ? 'bg-emerald-400'
                    : 'bg-gradient-to-r from-aether to-violet-400'
              }`}
              style={{ width: `${Math.min(100, Math.max(4, activity.percent))}%` }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}
