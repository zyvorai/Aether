// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { GitBranch, Loader2, Play, RefreshCw } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import Badge from './Badge';
import type { GitOpsAgentExecuteReport, GitOpsAgentSyncReport } from '../types/api';
import GlassSection from './GlassSection';

export default function GitOpsAgentPanel() {
  const [plan, setPlan] = useState<GitOpsAgentSyncReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<GitOpsAgentSyncReport>('/intelligence/gitops/agent/plan');
    setPlan(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function sync(dryRun: boolean) {
    if (!dryRun) {
      const ok = window.confirm('Execute GitOps agent sync? Drift reconcile requires autonomy policy.');
      if (!ok) return;
    }
    setExecuting(true);
    const res = await apiPost<GitOpsAgentExecuteReport>('/intelligence/gitops/agent/sync', { dry_run: dryRun });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `GitOps dry-run: ${res.data?.executed.length ?? 0} actions`
              : `GitOps sync: ${res.data?.executed.length ?? 0} executed`,
            type: 'success',
          },
        }),
      );
      void load();
    }
  }

  const prLinks = plan?.pr_links ?? [];

  return (
        <GlassSection
      accent="purple"
      testId="gitops-agent-panel"
      title="GitOps Agent"
      subtitle="Federation-aware drift reconcile loop"
      icon={<GitBranch className="h-5 w-5 text-violet-400" />}
      actions={<div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void sync(true)}
            disabled={executing}
            className="rounded-xl border border-violet-500/30 bg-violet-500/10 px-3 py-2 text-xs text-violet-200 hover:border-violet-400/50 disabled:opacity-60"
            data-testid="gitops-agent-dry-run"
          >
            Dry-run sync
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-muted hover:border-brand/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>}
    ><div className="mb-4 flex flex-wrap gap-2">
        {plan?.federation_enabled ? <Badge text="federation" variant="green" /> : <Badge text="local only" variant="muted" />}
        {plan?.federation_target ? <Badge text={plan.federation_target} variant="muted" /> : null}
        <Badge text={`${plan?.drift_workloads.length ?? 0} drifted`} variant="yellow" />
      </div>

      {!plan?.planned_actions.length ? (
        <p className="text-sm text-subtle">Loading agent plan…</p>
      ) : (
        <ul className="space-y-2">
          {plan.planned_actions.map((action) => (
            <li key={action} className="rounded-lg border glass-divider px-3 py-2 text-sm text-muted">
              <Play className="mr-1 inline h-3 w-3 text-violet-400" />
              {action}
            </li>
          ))}
        </ul>
      )}

      {prLinks.length > 0 ? (
        <div className="mt-4" data-testid="gitops-agent-pr-links">
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-subtle">Agent pull requests</p>
          <ul className="space-y-2">
            {prLinks.map((pr) => (
              <li key={pr.url} className="rounded-lg border glass-divider px-3 py-2 text-sm">
                <a href={pr.url} target="_blank" rel="noreferrer" className="text-brand hover:underline">
                  {pr.title}
                </a>
                <span className="ml-2 text-xs text-subtle">{pr.workload}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </GlassSection>
  );
}
