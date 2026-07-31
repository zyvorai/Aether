// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import { Inbox } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { apiFetchSettled } from '../../utils/api';
import BarChart from '../BarChart';
import EmptyState from '../EmptyState';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { AffinityScore } from '../../types/api';

const WORKLOAD_CLASSES = [
  'web-service',
  'database',
  'ml-training',
  'microservice',
  'api-backend',
  'cache',
  'batch-job',
  'worker',
];

export default function AffinityPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [affinityData, setAffinityData] = useState<Record<string, AffinityScore[]>>({});
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  type AffinityTab = 'recommend' | 'matrix' | 'stats';
  const [tabParam, setTabParam] = useQueryParam('tab', 'recommend');
  const [workloadQuery] = useQueryParam('workload', '');
  const workloadFocus = workloadQuery.trim() || undefined;
  const tab: AffinityTab = (['recommend', 'matrix', 'stats'] as const).includes(tabParam as AffinityTab)
    ? (tabParam as AffinityTab)
    : 'recommend';
  const setTab = (next: AffinityTab) => setTabParam(next);
  const [matrix, setMatrix] = useState<Array<{ class: string; runtime: string; compatible: boolean; score: number; deployments: number }>>([]);
  const [stats, setStats] = useState<Record<string, unknown> | null>(null);

  const load = useCallback(async () => {
    const results = await Promise.all(
      WORKLOAD_CLASSES.map(async (cls) => {
        const result = await apiFetchSettled<AffinityScore[]>(`/affinity/${cls}`);
        return { cls, result };
      }),
    );
    const merged: Record<string, AffinityScore[]> = {};
    let anyOk = false;
    for (const { cls, result } of results) {
      if (result.ok) {
        anyOk = true;
        merged[cls] = result.data;
      }
    }
    if (!anyOk) {
      setLoadFailed(true);
      setAffinityData({});
    } else {
      setLoadFailed(false);
      setAffinityData(merged);
    }
  }, [refreshKey]);

  useEffect(() => {
    void (async () => {
      await load();
      setLoading(false);
    })();
  }, [load]);

  async function handleRefresh() {
    setRefreshing(true);
    await load();
    if (tab === 'matrix') {
      const m = await apiFetchSettled<Array<{ class: string; runtime: string; compatible: boolean; score: number; deployments: number }>>('/affinity/matrix');
      if (m.ok) setMatrix(m.data);
    } else if (tab === 'stats') {
      const s = await apiFetchSettled<Record<string, unknown>>('/affinity/stats');
      if (s.ok) setStats(s.data);
    }
    setRefreshing(false);
  }

  useEffect(() => {
    if (tab === 'matrix') {
      void apiFetchSettled<Array<{ class: string; runtime: string; compatible: boolean; score: number; deployments: number }>>('/affinity/matrix').then((m) => {
        if (m.ok) setMatrix(m.data);
      });
    } else if (tab === 'stats') {
      void apiFetchSettled<Record<string, unknown>>('/affinity/stats').then((s) => {
        if (s.ok) setStats(s.data);
      });
    }
  }, [tab]);

  if (loading && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Affinity data unavailable" onRetry={() => void load()} />;
  }

  const classes = Object.entries(affinityData);

  function classMatchesWorkload(cls: string): boolean {
    if (!workloadFocus) return false;
    const normalized = workloadFocus.toLowerCase().replace(/[/]/g, '-');
    const classToken = cls.toLowerCase();
    return normalized.includes(classToken) || classToken.split('-').every((part) => normalized.includes(part));
  }

  return (
    <div>
      <PageToolbar onRefresh={() => void handleRefresh()} refreshing={refreshing} />

      {workloadFocus ? (
        <WorkloadContextBanner testId="affinity-workload-context" workload={workloadFocus} description="Affinity context">
          <WorkloadScopedCrossLinks workload={workloadFocus} prefix="affinity" showMetrics />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('scheduler'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="affinity-scheduler-scoped-link"
          >
            Scheduler →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('ai'), { workload: workloadFocus, tab: 'analyze' })}
            className="text-aether hover:underline"
            data-testid="affinity-ai-link"
          >
            AI engine →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('copilot'), { workload: workloadFocus, q: `Placement guidance for ${workloadFocus}` })}
            className="text-aether hover:underline"
            data-testid="affinity-context-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('cost'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="affinity-context-cost-link"
          >
            Cost →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('drift'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="affinity-context-drift-link"
          >
            Drift →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="affinity-context-platform-link"
          >
            Platform →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <section className="overview-section-shell mb-6 p-6 sm:p-8">
      <div className="flex gap-2 mb-6" data-testid="affinity-tabs">
        {(['recommend', 'matrix', 'stats'] as const).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={`glass-tab tab-chip capitalize ${tab === t ? 'glass-tab-active tab-chip-active' : ''}`}
          >
            {t === 'recommend' ? 'Recommendations' : t}
          </button>
        ))}
      </div>

      {tab === 'matrix' && (
        <div className="glass-panel-card overflow-x-auto mb-6" data-testid="affinity-matrix-panel">
          <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
            <h2 className="text-lg font-semibold text-slate-100">Runtime compatibility matrix</h2>
            <Link to={viewToPath('scheduler')} className="text-xs text-aether hover:underline" data-testid="affinity-scheduler-link">
              Placement scheduler →
            </Link>
          </div>
          <table className="w-full text-sm">
            <thead>
              <tr className="glass-divider-b text-xs uppercase text-slate-500">
                <th className="py-2 px-3 text-left">Class</th>
                <th className="py-2 px-3 text-left">Runtime</th>
                <th className="py-2 px-3 text-left">Compat</th>
                <th className="py-2 px-3 text-left">Score</th>
              </tr>
            </thead>
            <tbody>
              {matrix.map((row) => (
                <tr key={`${row.class}-${row.runtime}`} className="glass-table-row">
                  <td className="py-2 px-3">{row.class}</td>
                  <td className="py-2 px-3">
                    <Link
                      to={pathWithQuery(viewToPath('ai'), { tab: 'optimize' })}
                      className="text-aether hover:underline"
                    >
                      {row.runtime}
                    </Link>
                  </td>
                  <td className="py-2 px-3">{row.compatible ? '✓' : '✗'}</td>
                  <td className="py-2 px-3">{(row.score * 100).toFixed(0)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {tab === 'stats' && stats && (
        <div className="glass-panel-card mb-6" data-testid="affinity-stats-panel">
          <div className="mb-3 flex flex-wrap gap-2">
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { tab: 'place' })}
              className="text-xs text-aether hover:underline"
            >
              Placement intelligence →
            </Link>
          </div>
          <pre className="text-xs text-slate-300 overflow-x-auto">{JSON.stringify(stats, null, 2)}</pre>
        </div>
      )}

      {tab === 'recommend' && (
        classes.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No affinity data" description="Affinity scores are not available" />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 auto-rows-min" data-testid="affinity-recommend-panel">
            {classes.map(([cls, scores]) => {
              const top = [...scores].sort((a, b) => b.composite_score - a.composite_score)[0];
              return (
              <div
                key={cls}
                className={`glass-panel-card ${
                  classMatchesWorkload(cls) ? 'ring-1 ring-aether/40 border-aether/30' : ''
                }`}
                data-testid={classMatchesWorkload(cls) ? 'affinity-workload-highlight' : undefined}
              >
                <div className="flex flex-wrap items-center justify-between gap-2 mb-4">
                  <h2 className="text-lg font-semibold text-slate-100 capitalize">
                    {cls.replace(/-/g, ' ')}
                  </h2>
                  {top && (
                    <Link
                      to={pathWithQuery(viewToPath('scheduler'), {})}
                      className="text-xs text-aether hover:underline"
                    >
                      Scheduler placement →
                    </Link>
                  )}
                </div>
                <div className="space-y-3">
                  {scores.map((s) => (
                    <div key={s.runtime} className="space-y-1">
                      <BarChart
                        label={s.runtime}
                        percent={s.composite_score * 100}
                        detail={`${s.total_deployments} deploys`}
                      />
                      <div className="flex gap-3 text-xs text-slate-500 pl-1">
                        <span>Confidence: {(s.confidence * 100).toFixed(0)}%</span>
                        <span>Success: {(s.success_rate * 100).toFixed(0)}%</span>
                      </div>
                    </div>
                  ))}
                </div>
                {top && (
                  <Link
                    to={pathWithQuery(viewToPath('ai'), { tab: 'optimize' })}
                    className="mt-4 inline-flex text-xs text-aether hover:underline"
                  >
                    Compare {top.runtime} in AI engine →
                  </Link>
                )}
              </div>
            );
            })}
          </div>
        )
      )}
      </section>
    </div>
  );
}
