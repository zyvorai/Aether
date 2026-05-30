// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import { ArrowRight, ListChecks, Loader2 } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { viewToPath } from '../utils/dashboardRoutes';
import type { AppView, NextActionsReport } from '../types/api';
import { useNavigate } from 'react-router';

interface CommandCenterNextActionsProps {
  refreshKey?: number;
  onNavigate?: (view: AppView) => void;
}

function actionTone(actionType: string): string {
  switch (actionType) {
    case 'heal':
      return 'border-emerald-500/25 bg-emerald-500/5';
    case 'investigate':
      return 'border-red-500/25 bg-red-500/5';
    case 'optimize':
      return 'border-sky-500/25 bg-sky-500/5';
    case 'migrate':
    case 'place':
      return 'border-violet-500/25 bg-violet-500/5';
    case 'capacity':
      return 'border-amber-500/25 bg-amber-500/5';
    default:
      return 'border-slate-800/70 bg-slate-950/50';
  }
}

export default function CommandCenterNextActions({
  refreshKey = 0,
  onNavigate,
}: CommandCenterNextActionsProps) {
  const navigate = useNavigate();
  const [report, setReport] = useState<NextActionsReport | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    apiFetch<NextActionsReport>('/command-center/next-actions').then((data) => {
      if (!cancelled) {
        setReport(data);
        setLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  function go(route: string) {
    const view = route as AppView;
    if (onNavigate) {
      onNavigate(view);
    } else {
      navigate(viewToPath(view));
    }
  }

  if (loading && !report) {
    return (
      <div
        className="mb-8 animate-pulse rounded-[28px] border border-slate-800/60 bg-slate-950/40 p-6 backdrop-blur-xl"
        data-testid="command-center-next-actions"
      >
        <div className="h-6 w-40 rounded-lg bg-slate-800/80" />
        <div className="mt-4 space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-14 rounded-xl bg-slate-800/60" />
          ))}
        </div>
      </div>
    );
  }

  if (!report?.actions.length) return null;

  return (
    <section
      className="mb-8 overflow-hidden rounded-[28px] border border-slate-800/50 bg-slate-950/45 p-6 backdrop-blur-xl sm:p-8"
      data-testid="command-center-next-actions"
    >
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <ListChecks className="h-5 w-5 text-aether" />
          <div>
            <h3 className="text-lg font-semibold text-white">Next actions</h3>
            <p className="text-sm text-slate-400">Prioritized queue from fleet intelligence</p>
          </div>
        </div>
        {loading ? <Loader2 className="h-4 w-4 animate-spin text-slate-500" /> : null}
      </div>

      <ul className="space-y-2">
        {report.actions.map((action) => (
          <li key={action.id}>
            <button
              type="button"
              onClick={() => go(action.route)}
              className={`flex w-full items-start gap-3 rounded-xl border px-4 py-3 text-left transition hover:border-aether/40 ${actionTone(action.action_type)}`}
              data-testid={`next-action-${action.id}`}
            >
              <span className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-slate-900/80 text-[11px] font-semibold text-slate-400">
                {action.priority}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium text-white">{action.title}</span>
                <span className="mt-1 block text-xs text-slate-400">{action.detail}</span>
              </span>
              <ArrowRight className="mt-1 h-4 w-4 shrink-0 text-slate-500" />
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
