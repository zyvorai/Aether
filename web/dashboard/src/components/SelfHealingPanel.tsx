// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { HeartPulse, Loader2, Play, RefreshCw } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { viewToPath } from '../utils/dashboardRoutes';
import Badge from './Badge';
import type { HealerExecuteReport, HealerPreviewReport, RemediationPlan } from '../types/api';

export default function SelfHealingPanel() {
  const [preview, setPreview] = useState<HealerPreviewReport | null>(null);
  const [remediation, setRemediation] = useState<RemediationPlan | null>(null);
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const [healer, plan] = await Promise.all([
      apiFetch<HealerPreviewReport>('/intelligence/healer/preview'),
      apiFetch<RemediationPlan>('/intelligence/remediation/plan'),
    ]);
    setPreview(healer);
    setRemediation(plan);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function runRemediation(dryRun: boolean) {
    setExecuting(true);
    const res = await apiPost<{ executed: string[]; skipped: string[] }>('/intelligence/remediation/execute', {
      dry_run: dryRun,
      max_actions: 10,
    });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Dry-run: ${res.data?.executed?.length ?? 0} actions`
              : `Executed ${res.data?.executed?.length ?? 0} remediation actions`,
            type: 'success',
          },
        }),
      );
      void load();
    }
  }

  async function runHealer(dryRun: boolean) {
    if (!dryRun) {
      const ok = window.confirm(
        'Execute self-healing actions now? Live restarts require autonomy policy approval.',
      );
      if (!ok) return;
    }
    setExecuting(true);
    const res = await apiPost<HealerExecuteReport>('/intelligence/healer/execute', { dry_run: dryRun });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Healer dry-run: ${res.data?.executed?.length ?? 0} would execute`
              : `Healer executed ${res.data?.executed?.length ?? 0} actions`,
            type: 'success',
          },
        }),
      );
      void load();
    }
  }

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="self-healing-panel">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <HeartPulse className="h-5 w-5 text-red-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Self-Healing Orchestrator</h2>
            <p className="text-sm text-slate-400">
              Preview autonomous restarts, drift reconcile, and anomaly remediation before they run.
            </p>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void runHealer(true)}
            disabled={executing}
            className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-200 hover:border-emerald-400/50 disabled:opacity-60"
            data-testid="healer-dry-run"
          >
            Dry-run healer
          </button>
          <button
            type="button"
            onClick={() => void runHealer(false)}
            disabled={executing || !(preview?.would_execute.length ?? 0)}
            className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200 hover:border-red-400/50 disabled:opacity-60"
            data-testid="healer-execute"
          >
            Execute healer
          </button>
          <button
            type="button"
            onClick={() => void runRemediation(true)}
            disabled={executing}
            className="rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40 disabled:opacity-60"
          >
            Dry-run remediation
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>
      </div>

      <div className="mb-6 grid gap-3 sm:grid-cols-3">
        <div className="glass-metric-card">
          <div className="text-2xl font-semibold text-white">{preview?.would_execute.length ?? 0}</div>
          <div className="text-xs text-slate-500">Would execute</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-2xl font-semibold text-amber-200">{preview?.would_skip.length ?? 0}</div>
          <div className="text-xs text-slate-500">Blocked by policy</div>
        </div>
        <div className="glass-metric-card">
          <div className="text-2xl font-semibold text-white">{remediation?.actions.length ?? 0}</div>
          <div className="text-xs text-slate-500">Remediation actions queued</div>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <div>
          <p className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500">Healer preview</p>
          {!preview?.would_execute.length && !preview?.would_skip.length ? (
            <p className="text-sm text-slate-500">No pending healing actions.</p>
          ) : (
            <ul className="space-y-2">
              {preview?.would_execute.map((line) => (
                <li key={line} className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-3 py-2 text-sm text-emerald-100">
                  <Play className="mr-1 inline h-3 w-3" />
                  {line}
                </li>
              ))}
              {preview?.would_skip.map((line) => (
                <li key={line} className="rounded-lg border border-slate-800 px-3 py-2 text-sm text-slate-400">
                  {line}
                </li>
              ))}
            </ul>
          )}
        </div>

        <div>
          <p className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500">Remediation plan</p>
          {!remediation?.actions.length ? (
            <p className="text-sm text-slate-500">No anomaly or drift remediation queued.</p>
          ) : (
            <ul className="space-y-2">
              {remediation.actions.slice(0, 6).map((action) => (
                <li key={`${action.action_type}-${action.target}`} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-medium text-white">{action.action_type}</span>
                    <Badge text={action.target} variant="muted" />
                    {action.auto_safe ? <Badge text="auto-safe" variant="green" /> : null}
                  </div>
                  <p className="mt-1 text-xs text-slate-500">{action.reason}</p>
                </li>
              ))}
            </ul>
          )}
          <Link to={viewToPath('gitops')} className="mt-3 inline-block text-xs text-aether hover:underline">
            GitOps drift →
          </Link>
        </div>
      </div>
    </section>
  );
}
