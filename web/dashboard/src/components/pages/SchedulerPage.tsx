// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
import { Link, useNavigate } from 'react-router';
import { Inbox, MapPin } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatUSD } from '../../utils/formatters';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import StatCard from '../StatCard';
import BarChart from '../BarChart';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { RuntimeUtilization, OptimizeSuggestion } from '../../types/api';

function SchedulerPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [workloadQuery] = useQueryParam('workload');
  const [utilization, setUtilization] = useState<RuntimeUtilization[]>([]);
  const [suggestions, setSuggestions] = useState<OptimizeSuggestion[]>([]);
  const [placements, setPlacements] = useState<Array<{ workload_name: string; runtime: string; cpu_reserved: number; memory_reserved_mb: number; placed_at: string }>>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  const load = useCallback(async () => {
    setLoadFailed(false);
    const [u, s, p] = await Promise.all([
      apiFetchSettled<RuntimeUtilization[]>('/scheduler/utilization'),
      apiFetchSettled<OptimizeSuggestion[]>('/scheduler/optimize'),
      apiFetchSettled<Array<{ workload_name: string; runtime: string; cpu_reserved: number; memory_reserved_mb: number; placed_at: string }>>('/scheduler/placements'),
    ]);
    if (!u.ok && !s.ok && !p.ok) {
      setLoadFailed(true);
      setUtilization([]);
      setSuggestions([]);
      setPlacements([]);
    } else {
      setUtilization(u.ok ? u.data : []);
      setSuggestions(s.ok ? s.data : []);
      setPlacements(p.ok ? p.data : []);
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
    setRefreshing(false);
  }

  if (loading && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Scheduler data unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      {workloadQuery.trim() ? (
        <WorkloadContextBanner
          testId="scheduler-workload-context"
          workload={workloadQuery}
          openTestId="scheduler-open-workload"
          description="Placement context"
        >
          <WorkloadScopedCrossLinks workload={workloadQuery} prefix="scheduler" showDrift showGitops showMetrics />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('cost'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="scheduler-context-cost-link"
          >
            Cost →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('affinity'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="scheduler-affinity-link"
          >
            Affinity →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="scheduler-banner-fleet-link"
          >
            Fleet →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('intelligence'), { workload: workloadQuery.trim(), tab: 'place' })}
            className="text-brand hover:underline"
            data-testid="scheduler-context-intelligence-link"
          >
            Intelligence →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="scheduler-context-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="scheduler-context-openapi-link"
          >
            OpenAPI →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <section className="glass mb-6 space-y-6 p-6 sm:p-8">
      <PageToolbar onRefresh={() => void handleRefresh()} refreshing={refreshing} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6" data-testid="scheduler-utilization-panel">
        {utilization.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No utilization data" description="No runtimes are reporting utilization" />
        ) : (
          utilization.map((rt) => (
            <div key={rt.runtime} className="glass">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-semibold text-foreground capitalize">{rt.runtime}</h2>
                <div className="flex items-center gap-2">
                  <Badge
                    text={rt.healthy ? 'Healthy' : 'Unhealthy'}
                    variant={rt.healthy ? 'green' : 'red'}
                  />
                  <span className="text-xs text-subtle">{formatUSD(rt.estimated_cost_per_day)}/day</span>
                </div>
              </div>

              <div className="mb-4 flex items-center justify-between gap-2">
                <StatCard title="Workloads" value={`${rt.workload_count}/${rt.max_workloads}`} color="blue" />
                <Link
                  to={pathWithQuery(viewToPath('fleet'), {})}
                  className="text-xs text-brand hover:underline shrink-0"
                  data-testid="scheduler-fleet-link"
                >
                  Fleet →
                </Link>
              </div>

              <div className="space-y-3">
                <BarChart label="CPU Utilization" percent={rt.cpu_utilization * 100} />
                <BarChart label="Memory Utilization" percent={rt.memory_utilization * 100} />
              </div>
            </div>
          ))
        )}
      </div>

      <div className="mb-6" data-testid="scheduler-placements">
        <h2 className="mb-4 text-lg font-semibold text-foreground">Current placements</h2>
        {placements.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No placements" description="No scheduler placement records yet" />
        ) : (
          <CardGrid columns="compact">
            {placements.map((p, i) => (
              <EntityCard
                key={p.workload_name}
                index={i}
                testId={workloadQuery.trim() === p.workload_name ? 'scheduler-workload-highlight' : `scheduler-placement-${p.workload_name}`}
                icon={<MapPin size={18} />}
                statusTone="sky"
                selected={workloadQuery.trim() === p.workload_name}
                title={p.workload_name}
                subtitle={p.runtime}
                onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: p.workload_name }))}
                body={
                  <div className="flex flex-wrap gap-1.5">
                    <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">
                      CPU {p.cpu_reserved}
                    </span>
                    <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">
                      Mem {p.memory_reserved_mb} MB
                    </span>
                  </div>
                }
              />
            ))}
          </CardGrid>
        )}
      </div>

      <div className="glass" data-testid="scheduler-suggestions">
        <div className="flex items-center justify-between gap-3 mb-4">
          <h2 className="text-lg font-semibold text-foreground">Optimization Suggestions</h2>
          <div className="flex items-center gap-2">
            <Link to={viewToPath('affinity')} className="text-xs text-brand hover:underline">
              Runtime affinity →
            </Link>
            <Link to={viewToPath('cost')} className="text-xs text-brand hover:underline" data-testid="scheduler-cost-link">
              Cost estimation →
            </Link>
            <button
            type="button"
            data-testid="scheduler-refresh-optimize"
            onClick={() => void handleRefresh()}
            disabled={refreshing}
            className="text-xs rounded-lg border glass-divider px-3 py-1.5 text-muted hover:border-brand/40 disabled:opacity-50"
          >
            {refreshing ? 'Refreshing…' : 'Refresh suggestions'}
          </button>
          </div>
        </div>
        {suggestions.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No suggestions" description="No optimization suggestions at this time" />
        ) : (
          <div className="space-y-3">
            {suggestions.map((s, i) => (
              <div key={i} className="flex items-start gap-3 p-3 glass rounded-lg">
                <Badge text={s.category} variant="blue" />
                <div className="flex-1">
                  <div className="text-sm text-foreground">{s.message}</div>
                  {s.potential_saving !== null && (
                    <div className="text-xs text-emerald-400 mt-1">
                      Potential saving: {formatUSD(s.potential_saving)}/day
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      </section>
    </div>
  );
}

export default withAuroraPage('scheduler', SchedulerPage);
