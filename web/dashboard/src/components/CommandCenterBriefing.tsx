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
  Sparkles,
  TrendingUp,
} from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { syncMacOSDockBadge, syncMacOSTray } from '../utils/macosBridge';
import { formatUSD } from '../utils/formatters';
import type { AppView } from '../types/api';
import CommandMetricCard from './CommandMetricCard';

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
  clusterContext?: {
    connected: boolean;
    clusterCount: number;
    mode?: string;
  };
}

function severityTone(severity: string): string {
  const s = severity.toLowerCase();
  if (s === 'critical' || s === 'high') return 'border-red-500/30 bg-red-500/10 text-red-200';
  if (s === 'medium') return 'border-amber-500/30 bg-amber-500/10 text-amber-200';
  return 'glass-panel-card text-ink-2';
}

function contextualSubtitle(issueCount: number, fleetHealth: number): string {
  if (issueCount > 0) {
    return `${issueCount} signal${issueCount === 1 ? '' : 's'} need attention · Fleet at ${fleetHealth.toFixed(0)}% health`;
  }
  if (fleetHealth >= 99) {
    return 'All systems nominal · Intelligence layer is monitoring your fleet';
  }
  if (fleetHealth > 0) {
    return `Fleet operating at ${fleetHealth.toFixed(0)}% health · No critical signals detected`;
  }
  return 'Connect workloads to activate fleet intelligence and proactive monitoring';
}

export default function CommandCenterBriefing({ onNavigate, refreshKey = 0, clusterContext }: CommandCenterBriefingProps) {
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
      <div className="command-center-shell mb-8 animate-pulse p-6 sm:p-8">
        <div className="h-8 w-48 rounded-lg glass-inset-surface" />
        <div className="mt-2 h-4 w-72 rounded-lg glass-inset-surface" />
        <div className="mt-8 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="h-32 rounded-2xl glass-inset-surface" />
          ))}
        </div>
      </div>
    );
  }

  if (!briefing) return null;

  const issueCount = briefing.issues.filter((i) => i.severity !== 'info').length;
  const topCapacity = briefing.capacity_risks[0];
  const fleetEmpty = briefing.fleet_health_pct === 0;
  const issuesEmpty = issueCount === 0;
  const savingsEmpty = briefing.potential_savings_usd === 0;
  const migrationsEmpty = briefing.migration_opportunities === 0;

  return (
    <section className="command-center-shell mb-8 p-6 sm:p-8" data-testid="command-center-briefing">
      <div className="mb-8 flex flex-wrap items-end justify-between gap-4">
        <div className="min-w-0 flex-1">
          <div className="mb-3 flex items-center gap-2">
            <Sparkles className="h-4 w-4 text-brand" aria-hidden />
            <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-brand">Command Center</p>
          </div>
          <h2 className="text-3xl font-semibold tracking-tight text-ink sm:text-4xl">{briefing.greeting}</h2>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <p className="max-w-2xl text-sm leading-relaxed text-ink-2">
              {contextualSubtitle(issueCount, briefing.fleet_health_pct)}
            </p>
            {clusterContext ? (
              <span className="inline-flex items-center gap-1.5 rounded-full border border-brand/25 bg-brand/10 px-2.5 py-0.5 text-[11px] font-medium text-blue-200">
                {clusterContext.clusterCount > 0
                  ? `${clusterContext.clusterCount} cluster${clusterContext.clusterCount === 1 ? '' : 's'} · ${clusterContext.connected ? 'Live' : 'Offline'}`
                  : clusterContext.mode ?? 'Local mode · No kubeconfig'}
              </span>
            ) : null}
          </div>
        </div>
        <div className="live-intelligence-badge">
          <span className="live-intelligence-dot" />
          <span className="text-xs font-medium text-ink">Live Intelligence</span>
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <CommandMetricCard
          label="Fleet Health"
          value={`${briefing.fleet_health_pct.toFixed(0)}%`}
          icon={<Gauge className="h-3.5 w-3.5" />}
          accent={fleetEmpty ? 'muted' : 'emerald'}
          hint={fleetEmpty ? 'Deploy workloads to begin monitoring' : 'View health dashboard →'}
          isEmpty={fleetEmpty}
          onClick={() => onNavigate('health')}
          testId="briefing-fleet-health"
        >
          {!fleetEmpty ? (
            <div className="relative mt-4 h-1.5 glass-progress-track">
              <div
                className="h-full rounded-full bg-gradient-to-r from-emerald-400 via-aether to-aether-ai transition-all"
                style={{ width: `${Math.min(100, Math.max(4, briefing.fleet_health_pct))}%` }}
              />
            </div>
          ) : (
            <div className="relative mt-4 h-1.5 glass-progress-track">
              <div className="h-full w-1/4 rounded-full glass-inset-surface/80" />
            </div>
          )}
        </CommandMetricCard>

        <CommandMetricCard
          label="Issues Found"
          value={issueCount}
          icon={<AlertTriangle className="h-3.5 w-3.5" />}
          accent={issuesEmpty ? 'muted' : issueCount > 0 ? 'amber' : 'blue'}
          hint={issuesEmpty ? 'No active signals — fleet is calm' : 'View observability →'}
          isEmpty={issuesEmpty}
          onClick={() => onNavigate('observability')}
          testId="briefing-issues"
        />

        <CommandMetricCard
          label="Potential Savings"
          value={
            <>
              {formatUSD(briefing.potential_savings_usd)}
              <span className="ml-1 text-sm font-normal text-ink-3">/mo</span>
            </>
          }
          icon={<DollarSign className="h-3.5 w-3.5" />}
          accent={savingsEmpty ? 'muted' : 'emerald'}
          hint={savingsEmpty ? 'Cost optimization activates with workloads' : 'Open cost intelligence →'}
          isEmpty={savingsEmpty}
          onClick={() => onNavigate('cost')}
          testId="briefing-savings"
        />

        <CommandMetricCard
          label="Migration Opportunities"
          value={briefing.migration_opportunities}
          icon={<Rocket className="h-3.5 w-3.5" />}
          accent={migrationsEmpty ? 'muted' : 'purple'}
          hint={migrationsEmpty ? 'No placement changes recommended yet' : 'Review migration planner →'}
          isEmpty={migrationsEmpty}
          onClick={() => onNavigate('migrations')}
          testId="briefing-migrations"
        />
      </div>

      {topCapacity ? (
        <div
          className="mt-5 flex flex-wrap items-center gap-3 rounded-2xl border border-amber-500/25 bg-amber-500/10 px-4 py-3 backdrop-blur-sm"
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
            className="inline-flex items-center gap-1 text-xs font-medium text-amber-200 hover:text-ink"
          >
            Open Fabric
            <ArrowRight className="h-3.5 w-3.5" />
          </button>
        </div>
      ) : null}

      {briefing.issues.length > 0 ? (
        <div className="mt-8 space-y-3">
          <h3 className="text-[11px] font-semibold uppercase tracking-[0.18em] text-ink-3">Active signals</h3>
          <div className="grid gap-2 lg:grid-cols-2">
            {briefing.issues.slice(0, 4).map((issue, i) => (
              <div
                key={`${issue.title}-${i}`}
                className={`rounded-xl border px-4 py-3 backdrop-blur-sm ${severityTone(issue.severity)}`}
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
