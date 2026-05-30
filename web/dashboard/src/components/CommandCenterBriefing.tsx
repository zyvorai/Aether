// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import {
  AlertTriangle,
  ArrowRight,
  DollarSign,
  Gauge,
  Rocket,
  TrendingUp,
} from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { syncMacOSDockBadge, syncMacOSTray } from '../utils/macosBridge';
import { formatUSD } from '../utils/formatters';
import type { AppView } from '../types/api';

export interface CommandCenterBriefingData {
  generated_at: string;
  greeting: string;
  fleet_health_pct: number;
  issues: Array<{
    severity: string;
    title: string;
    detail: string;
    workload?: string | null;
  }>;
  potential_savings_usd: number;
  migration_opportunities: number;
  capacity_risks: Array<{
    resource: string;
    days_remaining: number;
    summary: string;
  }>;
}

interface CommandCenterBriefingProps {
  onNavigate: (view: AppView) => void;
  refreshKey?: number;
}

function severityTone(severity: string): string {
  const s = severity.toLowerCase();
  if (s === 'critical' || s === 'high') return 'border-red-500/30 bg-red-500/10 text-red-200';
  if (s === 'medium') return 'border-amber-500/30 bg-amber-500/10 text-amber-200';
  return 'border-slate-700/60 bg-slate-900/50 text-slate-300';
}

export default function CommandCenterBriefing({ onNavigate, refreshKey = 0 }: CommandCenterBriefingProps) {
  const [briefing, setBriefing] = useState<CommandCenterBriefingData | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    apiFetch<CommandCenterBriefingData>('/command-center/briefing').then(async (data) => {
      if (!cancelled) {
        setBriefing(data);
        setLoading(false);
        if (data) {
          const issues = data.issues.filter((i) => i.severity !== 'info').length;
          window.dispatchEvent(
            new CustomEvent('aether-tray-health', {
              detail: { fleet_health_pct: data.fleet_health_pct, issues },
            }),
          );
          const sparkline = await apiFetch<{ sparkline: string }>('/intelligence/macos/tray-sparkline').catch(
            () => null,
          );
          void syncMacOSTray(data.fleet_health_pct, issues, sparkline?.sparkline);
          void syncMacOSDockBadge(issues);
          void apiPost('/intelligence/macos/offline-cache').catch(() => {});
        }
      }
    });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  if (loading && !briefing) {
    return (
      <div className="mb-8 animate-pulse rounded-[28px] border border-slate-800/60 bg-slate-950/40 p-8 backdrop-blur-xl">
        <div className="h-8 w-48 rounded-lg bg-slate-800/80" />
        <div className="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="h-24 rounded-2xl bg-slate-800/60" />
          ))}
        </div>
      </div>
    );
  }

  if (!briefing) return null;

  const issueCount = briefing.issues.filter((i) => i.severity !== 'info').length;
  const topCapacity = briefing.capacity_risks[0];

  return (
    <section
      className="mb-8 overflow-hidden rounded-[28px] border border-slate-800/50 bg-slate-950/45 p-6 backdrop-blur-xl sm:p-8"
      data-testid="command-center-briefing"
    >
      <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-aether">Command Center</p>
          <h2 className="mt-2 text-3xl font-semibold tracking-tight text-white sm:text-4xl">{briefing.greeting}</h2>
        </div>
        <div className="flex items-center gap-2 rounded-full border border-emerald-500/25 bg-emerald-500/10 px-3 py-1.5">
          <span className="h-2 w-2 rounded-full bg-emerald-400 platform-pulse" />
          <span className="text-xs text-emerald-200">Live intelligence</span>
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <button
          type="button"
          onClick={() => onNavigate('health')}
          className="glass-metric-card group text-left"
          data-testid="briefing-fleet-health"
        >
          <div className="mb-2 flex items-center gap-2 text-[11px] uppercase tracking-[0.16em] text-slate-500">
            <Gauge className="h-3.5 w-3.5" />
            Fleet Health
          </div>
          <div className="text-3xl font-semibold text-white">{briefing.fleet_health_pct.toFixed(0)}%</div>
          <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-slate-800">
            <div
              className="h-full rounded-full bg-gradient-to-r from-emerald-400 to-aether transition-all"
              style={{ width: `${Math.min(100, briefing.fleet_health_pct)}%` }}
            />
          </div>
        </button>

        <button
          type="button"
          onClick={() => onNavigate('observability')}
          className="glass-metric-card group text-left"
          data-testid="briefing-issues"
        >
          <div className="mb-2 flex items-center gap-2 text-[11px] uppercase tracking-[0.16em] text-slate-500">
            <AlertTriangle className="h-3.5 w-3.5" />
            Issues Found
          </div>
          <div className="text-3xl font-semibold text-white">{issueCount}</div>
          <p className="mt-2 text-xs text-slate-500 group-hover:text-slate-400">View observability →</p>
        </button>

        <button
          type="button"
          onClick={() => onNavigate('cost')}
          className="glass-metric-card group text-left"
          data-testid="briefing-savings"
        >
          <div className="mb-2 flex items-center gap-2 text-[11px] uppercase tracking-[0.16em] text-slate-500">
            <DollarSign className="h-3.5 w-3.5" />
            Potential Savings
          </div>
          <div className="text-3xl font-semibold text-emerald-300">
            {formatUSD(briefing.potential_savings_usd)}
            <span className="text-sm font-normal text-slate-500">/mo</span>
          </div>
        </button>

        <button
          type="button"
          onClick={() => onNavigate('migrations')}
          className="glass-metric-card group text-left"
          data-testid="briefing-migrations"
        >
          <div className="mb-2 flex items-center gap-2 text-[11px] uppercase tracking-[0.16em] text-slate-500">
            <Rocket className="h-3.5 w-3.5" />
            Migration Opportunities
          </div>
          <div className="text-3xl font-semibold text-white">{briefing.migration_opportunities}</div>
        </button>
      </div>

      {topCapacity ? (
        <div
          className="mt-4 flex flex-wrap items-center gap-3 rounded-2xl border border-amber-500/25 bg-amber-500/10 px-4 py-3"
          data-testid="briefing-capacity-risk"
        >
          <TrendingUp className="h-4 w-4 shrink-0 text-amber-300" />
          <div className="min-w-0 flex-1 text-sm text-amber-100">
            <span className="font-medium">Predicted capacity risk — </span>
            {topCapacity.summary}
            <span className="text-amber-200/80"> · {topCapacity.days_remaining} days</span>
          </div>
          <button
            type="button"
            onClick={() => onNavigate('fabric')}
            className="inline-flex items-center gap-1 text-xs font-medium text-amber-200 hover:text-white"
          >
            Open Fabric
            <ArrowRight className="h-3.5 w-3.5" />
          </button>
        </div>
      ) : null}

      {briefing.issues.length > 0 ? (
        <div className="mt-6 space-y-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-[0.16em] text-slate-500">Active signals</h3>
          <div className="grid gap-2 lg:grid-cols-2">
            {briefing.issues.slice(0, 4).map((issue, i) => (
              <div
                key={`${issue.title}-${i}`}
                className={`rounded-xl border px-4 py-3 ${severityTone(issue.severity)}`}
              >
                <div className="text-sm font-medium">{issue.title}</div>
                <div className="mt-1 text-xs opacity-80">{issue.detail}</div>
              </div>
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}
