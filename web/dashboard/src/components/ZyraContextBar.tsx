// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { Sparkles, Zap } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { useQueryParam } from '../utils/urlState';

export interface ZyraInsightsReport {
  generated_at: string;
  summary: string;
  drift_workloads: number;
  security_alerts: number;
  cost_savings_usd: number;
  pending_actions: number;
  suggested_actions: string[];
}

interface ZyraContextBarProps {
  onAskZyra?: (prompt: string) => void;
  refreshKey?: number;
}

export default function ZyraContextBar({ onAskZyra, refreshKey = 0 }: ZyraContextBarProps) {
  const [workload] = useQueryParam('workload', '');
  const [insights, setInsights] = useState<ZyraInsightsReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    apiFetch<ZyraInsightsReport>('/zyra/insights').then((data) => {
      if (!cancelled) setInsights(data);
    });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  if (!insights) return null;

  return (
    <div
      className="border-b border-primary/20 bg-primary/[0.06] px-4 py-2 backdrop-blur-md lg:px-6"
      data-testid="zyra-context-bar"
    >
      <div className="mx-auto flex max-w-[1600px] flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-center gap-3">
          <Sparkles className="h-4 w-4 shrink-0 text-primary" aria-hidden />
          <div className="min-w-0">
            <p className="text-xs font-medium uppercase tracking-wider text-primary/90">Zyra</p>
            <p className="truncate text-sm text-foreground">{insights.summary}</p>
          </div>
          {workload.trim() ? (
            <span className="hidden rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-xs text-muted sm:inline">
              Workload: {workload.trim()}
            </span>
          ) : null}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {insights.suggested_actions.slice(0, 2).map((action) => (
            <button
              key={action}
              type="button"
              onClick={() => onAskZyra?.(action)}
              className="rounded-lg border border-primary/25 bg-primary/10 px-2.5 py-1 text-xs text-primary hover:border-primary/40"
            >
              {action}
            </button>
          ))}
          <button
            type="button"
            onClick={() => onAskZyra?.('Summarize fleet health and recommend next actions')}
            className="inline-flex items-center gap-1 rounded-lg border border-white/10 px-2.5 py-1 text-xs text-muted hover:border-primary/30 hover:text-foreground"
            data-testid="zyra-context-explain"
          >
            <Zap className="h-3 w-3" />
            Ask Zyra
          </button>
        </div>
      </div>
    </div>
  );
}
