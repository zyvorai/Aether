// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import {
  ArrowRight,
  Bot,
  DollarSign,
  HeartPulse,
  ListChecks,
  Loader2,
  Rocket,
  Search,
  Sparkles,
  TrendingUp,
} from 'lucide-react';
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
      return 'border-emerald-500/20 bg-emerald-500/[0.06]';
    case 'investigate':
      return 'border-red-500/20 bg-red-500/[0.06]';
    case 'optimize':
      return 'border-brand/20 bg-brand/[0.06]';
    case 'migrate':
    case 'place':
      return 'border-aether-ai/20 bg-aether-ai/[0.06]';
    case 'capacity':
      return 'border-amber-500/20 bg-amber-500/[0.06]';
    default:
      return 'glass-divider glass-inset-surface';
  }
}

function actionIcon(actionType: string) {
  switch (actionType) {
    case 'heal':
      return HeartPulse;
    case 'investigate':
      return Search;
    case 'optimize':
      return DollarSign;
    case 'migrate':
    case 'place':
      return Rocket;
    case 'capacity':
      return TrendingUp;
    default:
      return Bot;
  }
}

function actionTypeLabel(actionType: string): string {
  switch (actionType) {
    case 'heal':
      return 'Heal';
    case 'investigate':
      return 'Investigate';
    case 'optimize':
      return 'Optimize';
    case 'migrate':
      return 'Migrate';
    case 'place':
      return 'Place';
    case 'capacity':
      return 'Capacity';
    default:
      return 'Action';
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
        className="glass mb-8 animate-pulse p-6 sm:p-8"
        data-testid="command-center-next-actions"
      >
        <div className="h-6 w-40 rounded-lg glass-inset-surface" />
        <div className="mt-5 space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-16 rounded-2xl glass-inset-surface" />
          ))}
        </div>
      </div>
    );
  }

  const actions = report?.actions ?? [];

  return (
    <section className="glass mb-8 p-6 sm:p-8" data-testid="command-center-next-actions">
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl border border-brand/25 bg-brand/10">
            <ListChecks className="h-5 w-5 text-brand" />
          </div>
          <div>
            <h3 className="text-lg font-semibold text-foreground">Next actions</h3>
            <p className="text-sm text-muted">Prioritized queue from fleet intelligence</p>
          </div>
        </div>
        {loading ? <Loader2 className="h-4 w-4 animate-spin text-subtle" /> : null}
      </div>

      {actions.length === 0 ? (
        <div className="rounded-2xl border border-dashed glass-divider glass-inset-surface px-6 py-10 text-center backdrop-blur-sm">
          <Sparkles className="mx-auto mb-3 h-8 w-8 text-subtle" />
          <p className="text-sm font-medium text-muted">Queue is clear</p>
          <p className="mt-1 text-xs text-subtle">
            No prioritized actions right now. Intelligence will surface recommendations as your fleet grows.
          </p>
        </div>
      ) : (
        <ul className="space-y-2.5">
          {actions.map((action) => {
            const Icon = actionIcon(action.action_type);
            return (
              <li key={action.id}>
                <button
                  type="button"
                  onClick={() => go(action.route)}
                  className={`next-action-card flex w-full items-start gap-3 ${actionTone(action.action_type)}`}
                  data-testid={`next-action-${action.id}`}
                >
                  <span className="next-action-priority">{action.priority}</span>
                  <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border border-white/8 bg-white/[0.04]">
                    <Icon className="h-4 w-4 text-muted" />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="flex flex-wrap items-center gap-2">
                      <span className="block text-sm font-medium text-foreground">{action.title}</span>
                      <span className="rounded-full border quick-link-chip px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-subtle">
                        {actionTypeLabel(action.action_type)}
                      </span>
                    </span>
                    <span className="mt-1 block text-xs leading-relaxed text-muted">{action.detail}</span>
                  </span>
                  <ArrowRight className="mt-2 h-4 w-4 shrink-0 text-subtle transition group-hover:text-brand" />
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
