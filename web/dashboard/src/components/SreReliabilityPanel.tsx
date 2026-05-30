// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import {
  Activity,
  AlertTriangle,
  Clock,
  Loader2,
  Phone,
  RefreshCw,
  Shield,
  Zap,
} from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import Badge from './Badge';
import type {
  ChaosExperimentCatalog,
  ErrorBudgetDashboardReport,
  EscalationPolicyReport,
  GameDayPlanReport,
  IncidentTimelineReport,
  MttrReport,
  OnCallIntegrationReport,
  PostmortemReport,
  RunbookExecuteReport,
  SreRunbookScheduleReport,
} from '../types/api';

type Tab = 'timeline' | 'budget' | 'postmortem' | 'oncall' | 'escalation' | 'mttr' | 'chaos' | 'gamedays';

export default function SreReliabilityPanel() {
  const [tab, setTab] = useState<Tab>('timeline');
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);
  const [schedule, setSchedule] = useState<SreRunbookScheduleReport | null>(null);
  const [timeline, setTimeline] = useState<IncidentTimelineReport | null>(null);
  const [budgets, setBudgets] = useState<ErrorBudgetDashboardReport | null>(null);
  const [postmortem, setPostmortem] = useState<PostmortemReport | null>(null);
  const [onCall, setOnCall] = useState<OnCallIntegrationReport | null>(null);
  const [escalation, setEscalation] = useState<EscalationPolicyReport | null>(null);
  const [mttr, setMttr] = useState<MttrReport | null>(null);
  const [chaos, setChaos] = useState<ChaosExperimentCatalog | null>(null);
  const [gameDays, setGameDays] = useState<GameDayPlanReport | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [sch, tl, eb, pm, oc, esc, mt, ch, gd] = await Promise.all([
      apiFetch<SreRunbookScheduleReport>('/intelligence/sre/schedule'),
      apiFetch<IncidentTimelineReport>('/intelligence/sre/incident-timeline'),
      apiFetch<ErrorBudgetDashboardReport>('/intelligence/sre/error-budgets'),
      apiFetch<PostmortemReport>('/intelligence/sre/postmortem'),
      apiFetch<OnCallIntegrationReport>('/intelligence/sre/on-call'),
      apiFetch<EscalationPolicyReport>('/intelligence/sre/escalation'),
      apiFetch<MttrReport>('/intelligence/sre/mttr'),
      apiFetch<ChaosExperimentCatalog>('/intelligence/sre/chaos/experiments'),
      apiFetch<GameDayPlanReport>('/intelligence/sre/game-days'),
    ]);
    setSchedule(sch);
    setTimeline(tl);
    setBudgets(eb);
    setPostmortem(pm);
    setOnCall(oc);
    setEscalation(esc);
    setMttr(mt);
    setChaos(ch);
    setGameDays(gd);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function executeRunbook() {
    setExecuting(true);
    const res = await apiPost<RunbookExecuteReport>('/intelligence/sre/runbook/execute', { dry_run: true });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: `Runbook dry-run: ${res.data?.executed.length ?? 0} steps`,
            type: 'info',
          },
        }),
      );
    }
  }

  async function testOnCall() {
    setExecuting(true);
    const res = await apiPost<{ message: string }>('/intelligence/sre/on-call/test', {
      dry_run: true,
      provider: 'pagerduty',
    });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', { detail: { message: res.data?.message ?? 'Test sent', type: 'success' } }),
      );
    }
  }

  async function runChaos(id: string) {
    setExecuting(true);
    const res = await apiPost<{ executed: string[] }>('/intelligence/sre/chaos/run', {
      dry_run: true,
      experiment_id: id,
    });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: { message: `Chaos dry-run: ${id}`, type: 'info' },
        }),
      );
    }
  }

  const tabs: { id: Tab; label: string }[] = [
    { id: 'timeline', label: 'Incidents' },
    { id: 'budget', label: 'Error budget' },
    { id: 'postmortem', label: 'Postmortem' },
    { id: 'oncall', label: 'On-call' },
    { id: 'escalation', label: 'Escalation' },
    { id: 'mttr', label: 'MTTR' },
    { id: 'chaos', label: 'Chaos' },
    { id: 'gamedays', label: 'Game days' },
  ];

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="sre-reliability-panel">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Shield className="h-5 w-5 text-rose-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">SRE Reliability Platform</h2>
            <p className="text-sm text-slate-400">
              Incident timeline, error budgets, postmortems, on-call, escalation, and MTTR
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void executeRunbook()}
            disabled={executing}
            className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-200 disabled:opacity-60"
            data-testid="runbook-execute-dry-run"
          >
            Execute runbook (dry-run)
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>
      </div>

      {schedule ? (
        <div className="mb-4 flex flex-wrap gap-2" data-testid="sre-runbook-schedule">
          <Badge text={schedule.scheduler_enabled ? 'scheduler on' : 'scheduler off'} variant="muted" />
          {schedule.entries.slice(0, 2).map((e) => (
            <Badge key={e.id} text={`${e.label}: ${e.cron}`} variant="muted" />
          ))}
        </div>
      ) : null}

      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-rose-500/40 bg-rose-500/10 text-rose-200' : 'border-slate-700 text-slate-400'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'timeline' ? (
        <ul className="space-y-2" data-testid="incident-timeline-panel">
          {(timeline?.entries ?? []).slice(0, 12).map((e) => (
            <li key={e.id} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
              <div className="flex flex-wrap items-center gap-2">
                <Clock className="h-3.5 w-3.5 text-slate-500" />
                <span className="text-xs text-slate-500">{e.timestamp}</span>
                <Badge text={e.severity} variant={e.severity === 'critical' ? 'red' : 'muted'} />
                <span className="font-medium text-white">{e.title}</span>
              </div>
              <p className="mt-1 text-xs text-slate-400">{e.detail}</p>
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'budget' ? (
        <ul className="space-y-2" data-testid="error-budget-panel">
          {(budgets?.entries ?? []).slice(0, 8).map((e) => (
            <li key={e.workload} className="rounded-lg border border-slate-800 px-3 py-2 text-sm text-slate-300">
              <div className="flex flex-wrap items-center gap-2">
                <Activity className="h-3.5 w-3.5" />
                <span className="font-medium text-white">{e.workload}</span>
                <Badge text={e.status} variant={e.status.includes('VIOLATED') ? 'red' : 'green'} />
                <span className="text-xs text-slate-500">
                  {e.error_budget.consumed_pct.toFixed(0)}% budget consumed · burn {e.burn_rate.toFixed(2)}/day
                </span>
              </div>
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'postmortem' ? (
        <div data-testid="postmortem-panel">
          <p className="mb-3 text-sm font-medium text-white">{postmortem?.title}</p>
          {(postmortem?.sections ?? []).map((s) => (
            <div key={s.heading} className="mb-4">
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-500">{s.heading}</h3>
              <ul className="space-y-1">
                {s.bullets.map((b) => (
                  <li key={b} className="text-sm text-slate-400">
                    {b}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      ) : null}

      {tab === 'oncall' ? (
        <div data-testid="on-call-panel">
          <button
            type="button"
            onClick={() => void testOnCall()}
            disabled={executing}
            className="mb-4 rounded-xl border border-blue-500/30 bg-blue-500/10 px-3 py-2 text-xs text-blue-200"
          >
            Test PagerDuty (dry-run)
          </button>
          <ul className="space-y-2">
            {(onCall?.channels ?? []).map((c) => (
              <li key={c.provider} className="flex items-center gap-2 rounded-lg border border-slate-800 px-3 py-2 text-sm">
                <Phone className="h-4 w-4 text-slate-400" />
                <span className="text-white">{c.provider}</span>
                <Badge text={c.configured ? 'linked' : 'missing'} variant={c.configured ? 'green' : 'muted'} />
                <span className="text-xs text-slate-500">{c.env_hint}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'escalation' ? (
        <ol className="space-y-2" data-testid="escalation-panel">
          {(escalation?.policies ?? []).map((p) => (
            <li key={p.order} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
              <span className="font-medium text-white">
                {p.order}. {p.agent}
              </span>
              <p className="text-xs text-slate-400">
                {p.trigger} → {p.action}
              </p>
            </li>
          ))}
        </ol>
      ) : null}

      {tab === 'mttr' ? (
        <div data-testid="mttr-panel">
          <p className="mb-3 text-sm text-emerald-300">
            Fleet avg MTTR: {(mttr?.fleet_avg_mttr_minutes ?? 0).toFixed(1)} min
          </p>
          <ul className="space-y-2">
            {(mttr?.entries ?? []).slice(0, 6).map((e) => (
              <li key={e.workload} className="rounded-lg border border-slate-800 px-3 py-2 text-sm text-slate-300">
                {e.workload}: {e.incidents} incidents · avg {e.avg_recovery_minutes.toFixed(0)} min
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'chaos' ? (
        <ul className="space-y-2" data-testid="chaos-experiments-panel">
          {(chaos?.experiments ?? []).slice(0, 6).map((e) => (
            <li key={e.id} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-slate-800 px-3 py-2 text-sm">
              <div>
                <span className="font-medium text-white">{e.label}</span>
                <Badge text={e.risk} variant={e.risk === 'high' ? 'red' : 'yellow'} />
                <p className="text-xs text-slate-500">{e.description}</p>
              </div>
              <button
                type="button"
                onClick={() => void runChaos(e.id)}
                disabled={executing}
                className="rounded-lg border border-amber-500/30 px-2 py-1 text-xs text-amber-200"
              >
                Dry-run
              </button>
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'gamedays' ? (
        <ul className="space-y-3" data-testid="game-days-panel">
          {(gameDays?.scenarios ?? []).map((s) => (
            <li key={s.id} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
              <div className="mb-2 flex items-center gap-2">
                <Zap className="h-4 w-4 text-violet-400" />
                <span className="font-medium text-white">{s.title}</span>
                <Badge text={`${s.duration_minutes}m`} variant="muted" />
              </div>
              <ul className="ml-6 list-disc text-xs text-slate-400">
                {s.steps.map((step) => (
                  <li key={step}>{step}</li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      ) : null}

      {!loading && tab === 'timeline' && !(timeline?.entries.length ?? 0) ? (
        <p className="flex items-center gap-2 text-sm text-slate-500">
          <AlertTriangle className="h-4 w-4" />
          No incidents in timeline yet.
        </p>
      ) : null}

      {!loading && tab === 'postmortem' && postmortem ? (
        <details className="mt-4">
          <summary className="cursor-pointer text-sm text-aether">View markdown</summary>
          <pre className="mt-2 max-h-48 overflow-auto rounded-xl border border-slate-800 bg-black/40 p-3 text-xs text-slate-300">
            {postmortem.markdown}
          </pre>
        </details>
      ) : null}
    </section>
  );
}
