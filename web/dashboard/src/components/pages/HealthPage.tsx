import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import { Inbox, HeartPulse } from 'lucide-react';
import { apiFetch, apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import { useAuth } from '../../contexts/AuthContext';
import { markHealthReviewed } from '../../utils/onboardingState';
import PageToolbar from '../PageToolbar';
import StatRibbon from '../StatRibbon';
import CardGrid from '../CardGrid';
import EntityCard, { type EntityStatusTone } from '../EntityCard';
import Badge, { RuntimeBadge } from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { HealthSummary, ManagedWorkload, HealthHistorySummary } from '../../types/api';
import { SectionHeader } from '../layout/SectionHeader';

function getHealthVariant(health: string): 'green' | 'yellow' | 'red' | 'muted' {
  const h = health.toLowerCase();
  if (h === 'healthy') return 'green';
  if (h === 'degraded') return 'yellow';
  if (h === 'unhealthy') return 'red';
  return 'muted';
}

function getCircuitVariant(circuit: string): 'green' | 'red' | 'yellow' | 'muted' {
  const c = circuit.toLowerCase();
  if (c === 'closed') return 'green';
  if (c === 'open') return 'red';
  if (c === 'half-open' || c === 'half_open') return 'yellow';
  return 'muted';
}

function getHealthTone(health: string): EntityStatusTone {
  const h = health.toLowerCase();
  if (h === 'healthy') return 'green';
  if (h === 'degraded') return 'amber';
  if (h === 'unhealthy') return 'red';
  return 'muted';
}

function HealthPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [summary, setSummary] = useState<HealthSummary | null>(null);
  const [workloads, setWorkloads] = useState<ManagedWorkload[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [statusFilter, setStatusFilter] = useQueryParam('status', 'all');
  const [workloadParam, setWorkloadParam] = useQueryParam('workload');
  const [selected, setSelected] = useState<{ workload: ManagedWorkload; history: HealthHistorySummary } | null>(null);
  const [historyLoading, setHistoryLoading] = useState<string | null>(null);
  const [orchBusy, setOrchBusy] = useState<string | null>(null);
  const [rollingReplicas, setRollingReplicas] = useState('2');
  const [rollingMsg, setRollingMsg] = useState<string | null>(null);
  const { canMutate } = useAuth();

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [s, w] = await Promise.all([
      apiFetchSettled<HealthSummary>('/orchestrator/summary'),
      apiFetchSettled<ManagedWorkload[]>('/orchestrator/status'),
    ]);
    if (!s.ok && !w.ok) {
      setLoadFailed(true);
      setSummary(null);
      setWorkloads([]);
    } else {
      setSummary(s.ok ? s.data : null);
      setWorkloads(w.ok ? w.data : []);
    }
    setLoading(false);
    if (s.ok || w.ok) {
      markHealthReviewed();
    }
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!workloadParam || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === workloadParam);
    if (match && selected?.workload.name !== match.name) {
      void handleRowClick(match);
    }
  }, [workloadParam, workloads, selected?.workload.name]);

  async function handleRowClick(w: ManagedWorkload) {
    if (selected?.workload.name === w.name) {
      setSelected(null);
      return;
    }
    setHistoryLoading(w.name);
    const data = await apiFetch<HealthHistorySummary>(`/health/${w.name}`);
    setHistoryLoading(null);
    if (data) {
      setSelected({ workload: w, history: data });
    }
  }

  async function runOrchestratorAction(action: 'health-check' | 'reset-circuit', name?: string) {
    if (!canMutate) return;
    setOrchBusy(action);
    if (action === 'health-check') {
      await apiPost('/orchestrator/health-check', {});
    } else if (name) {
      await apiPost('/orchestrator/reset-circuit', { name });
    }
    setOrchBusy(null);
    void load();
  }

  async function runRollingUpdate(name: string) {
    if (!canMutate) return;
    const replicas = parseInt(rollingReplicas, 10);
    if (!Number.isFinite(replicas) || replicas < 1) {
      setRollingMsg('Replicas must be a positive integer');
      return;
    }
    setOrchBusy('rolling-update');
    setRollingMsg(null);
    const res = await apiPost<{ message?: string }>('/orchestrator/rolling-update', { name, replicas });
    setOrchBusy(null);
    if (res.success) {
      setRollingMsg(`Rolling update started for ${name} (${replicas} replicas)`);
      void load();
    } else {
      setRollingMsg(res.error ?? 'Rolling update failed');
    }
  }

  const filtered = workloads.filter((w) => {
    const matchesSearch = w.name.toLowerCase().includes(search.toLowerCase());
    const status = statusFilter.toLowerCase();
    const matchesStatus = status === 'all' || (w.health ?? '').toLowerCase() === status;
    return matchesSearch && matchesStatus;
  });

  const hubBanner = (
    <div className="mb-6 glass-context-banner" data-testid="health-hub-context">
      Orchestrator
      {' · '}
      <Link to={viewToPath('fleet')} className="text-brand hover:underline" data-testid="health-context-fleet-link">
        Fleet →
      </Link>
      {' · '}
      <Link
        to={viewToPath('intelligence')}
        className="text-brand hover:underline"
        data-testid="health-context-intelligence-hub-link"
      >
        Intelligence →
      </Link>
      {' · '}
      <Link
        to={`${viewToPath('fleet')}?tab=edge`}
        className="text-brand hover:underline"
        data-testid="health-context-edge-link"
      >
        Edge →
      </Link>
      {' · '}
      <Link to={viewToPath('settings')} className="text-brand hover:underline" data-testid="health-context-settings-link">
        Identity & SSO →
      </Link>
    </div>
  );

  if (loading && workloads.length === 0 && !loadFailed) {
    return (
      <div>
        {hubBanner}
        <PageLoading rows={5} />
      </div>
    );
  }

  if (loadFailed) {
    return (
      <div>
        {hubBanner}
        <PageLoadError title="Health data unavailable" onRetry={() => void load()} />
      </div>
    );
  }

  return (
    <div>
      {hubBanner}
      <WorkloadContextBanner
        testId="health-workload-context"
        workload={workloadParam}
        description="Health monitor for workload"
      >
        <WorkloadScopedCrossLinks
          workload={workloadParam}
          prefix="health"
          showDrift
          showGitops
          showMetrics
        />
        {workloadParam.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('platform'), { workload: workloadParam.trim() })}
              className="text-brand hover:underline"
              data-testid="health-platform-link"
            >
              Platform →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: workloadParam.trim() })}
              className="text-brand hover:underline"
              data-testid="health-openapi-link"
            >
              OpenAPI →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('zyra'), { workload: workloadParam.trim(), q: `Why is ${workloadParam.trim()} unhealthy?` })}
              className="text-brand hover:underline"
              data-testid="health-copilot-link"
            >
              Copilot →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: workloadParam.trim() })}
              className="text-brand hover:underline"
              data-testid="health-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: workloadParam.trim() })}
              className="text-brand hover:underline"
              data-testid="health-context-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { workload: workloadParam.trim(), tab: 'predictions' })}
              className="text-brand hover:underline"
              data-testid="health-context-intelligence-link"
            >
              Intelligence →
            </Link>
          </>
        ) : null}
      </WorkloadContextBanner>
      {summary && (
        <section className="glass mb-6 p-6 sm:p-8">
          <SectionHeader
            label="Health"
            title="Fleet status"
            description="Orchestrator health checks and circuit breaker state"
          />
        <StatRibbon
          columns={5}
          items={[
            { label: 'Healthy', value: summary.healthy, tone: 'emerald', onClick: () => setStatusFilter('healthy'), testId: 'health-healthy-stat' },
            { label: 'Degraded', value: summary.degraded, tone: 'amber', onClick: () => setStatusFilter('degraded'), testId: 'health-degraded-stat' },
            { label: 'Unhealthy', value: summary.unhealthy, tone: 'red', onClick: () => setStatusFilter('unhealthy'), testId: 'health-unhealthy-stat' },
            { label: 'Unknown', value: summary.unknown, tone: 'sky', onClick: () => setStatusFilter('unknown'), testId: 'health-unknown-stat' },
            { label: 'Circuits open', value: summary.circuits_open, tone: 'aether' },
          ]}
        />
        </section>
      )}

      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Filter workloads…"
        onRefresh={() => void load()}
        refreshing={loading || orchBusy !== null}
        actions={
          canMutate ? (
            <button
              type="button"
              data-testid="health-run-checks"
              onClick={() => void runOrchestratorAction('health-check')}
              disabled={orchBusy !== null}
              className="rounded-xl bg-emerald-600/20 border border-emerald-500/40 px-4 py-2 text-sm text-emerald-300 hover:bg-emerald-600/30 disabled:opacity-50"
            >
              {orchBusy === 'health-check' ? 'Checking…' : 'Run health checks'}
            </button>
          ) : null
        }
        filters={
          <>
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="glass-input"
            aria-label="Health status filter"
            data-testid="health-status-filter"
          >
            <option value="all">All statuses</option>
            <option value="healthy">Healthy</option>
            <option value="degraded">Degraded</option>
            <option value="unhealthy">Unhealthy</option>
            <option value="unknown">Unknown</option>
          </select>
          {statusFilter !== 'all' && (
            <button
              type="button"
              data-testid="health-clear-filter"
              onClick={() => setStatusFilter('all')}
              className="rounded-lg border glass-divider px-3 py-2 text-xs text-muted hover:text-brand"
            >
              Clear filter
            </button>
          )}
          </>
        }
      />

      {workloads.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No managed workloads" description="No workloads are being monitored" />
      ) : (
        <>
          {filtered.length === 0 ? (
            <EmptyState icon={<Inbox size={48} />} title="No matching workloads" description="No workloads match your search." />
          ) : (
            <CardGrid columns="compact" className="mb-6" testId="health-workload-table">
              {filtered.map((w, i) => (
                <EntityCard
                  key={w.name}
                  index={i}
                  testId={`health-card-${w.name}`}
                  icon={<HeartPulse size={18} />}
                  statusTone={getHealthTone(w.health)}
                  pulse={w.health.toLowerCase() === 'healthy'}
                  selected={selected?.workload.name === w.name}
                  title={historyLoading === w.name ? 'Loading…' : w.name}
                  badge={<Badge text={w.health} variant={getHealthVariant(w.health)} />}
                  onClick={() => void handleRowClick(w)}
                  body={
                    <div className="flex flex-wrap items-center gap-1.5">
                      <RuntimeBadge runtime={w.runtime} />
                      <Badge text={w.circuit ?? 'unknown'} variant={getCircuitVariant(w.circuit ?? 'unknown')} />
                      <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">{w.restart_count} restarts</span>
                    </div>
                  }
                />
              ))}
            </CardGrid>
          )}

          {selected && (
            <div className="glass" data-testid="health-detail-panel">
              <h3 className="text-lg font-semibold text-foreground mb-4">
                Health detail:{' '}
                <button
                  type="button"
                  onClick={() =>
                    navigate(pathWithQuery(viewToPath('workloads'), { workload: selected.workload.name }))
                  }
                  className="text-brand hover:underline"
                >
                  {selected.workload.name}
                </button>
              </h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                <div className="glass py-3">
                  <div className="text-xs text-subtle mb-1">Total checks</div>
                  <div className="text-lg font-semibold text-foreground">{selected.history.total_checks}</div>
                </div>
                <div className="glass py-3">
                  <div className="text-xs text-subtle mb-1">Ready checks</div>
                  <div className="text-lg font-semibold text-emerald-400">{selected.history.ready_checks}</div>
                </div>
                <div className="glass py-3">
                  <div className="text-xs text-subtle mb-1">Uptime</div>
                  <div className="text-lg font-semibold text-foreground">{selected.history.uptime_percent.toFixed(2)}%</div>
                </div>
                <div className="glass py-3">
                  <div className="text-xs text-subtle mb-1">Last state</div>
                  <div className="text-lg font-semibold text-foreground">{selected.history.last_state}</div>
                </div>
              </div>
              <div className="flex flex-wrap gap-4 text-sm text-muted">
                <span>
                  Runtime: <span className="text-foreground">{selected.workload.runtime}</span>
                </span>
                <span>
                  Circuit: <Badge text={selected.workload.circuit} variant={getCircuitVariant(selected.workload.circuit)} />
                </span>
                {canMutate && selected.workload.circuit.toLowerCase() !== 'closed' && (
                  <button
                    type="button"
                    onClick={() => void runOrchestratorAction('reset-circuit', selected.workload.name)}
                    className="text-xs rounded-lg border border-amber-500/40 px-2 py-1 text-amber-300 hover:bg-amber-500/10"
                  >
                    Reset circuit
                  </button>
                )}
                {canMutate && (
                  <div className="flex flex-wrap items-center gap-2" data-testid="health-rolling-update">
                    <label className="text-xs text-subtle" htmlFor="rolling-replicas">
                      Rolling update replicas
                    </label>
                    <input
                      id="rolling-replicas"
                      type="number"
                      min={1}
                      value={rollingReplicas}
                      onChange={(e) => setRollingReplicas(e.target.value)}
                      className="glass-input w-16 text-xs"
                    />
                    <button
                      type="button"
                      data-testid="health-rolling-update-submit"
                      onClick={() => void runRollingUpdate(selected.workload.name)}
                      disabled={orchBusy !== null}
                      className="text-xs rounded-lg border border-emerald-500/40 px-2 py-1 text-emerald-300 hover:bg-emerald-500/10 disabled:opacity-50"
                    >
                      {orchBusy === 'rolling-update' ? 'Updating…' : 'Rolling update'}
                    </button>
                  </div>
                )}
                {rollingMsg ? <span className="text-xs text-muted">{rollingMsg}</span> : null}
                <Link
                  to={pathWithQuery(viewToPath('events'), { workload: selected.workload.name })}
                  className="text-xs text-brand hover:underline"
                >
                  View events →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('gitops'), { workload: selected.workload.name })}
                  className="text-xs text-brand hover:underline"
                  data-testid="health-gitops-link"
                >
                  GitOps →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('drift'), { workload: selected.workload.name })}
                  className="text-xs text-brand hover:underline"
                  data-testid="health-drift-link"
                >
                  Drift →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('alerts'), { workload: selected.workload.name })}
                  className="text-xs text-brand hover:underline"
                  data-testid="health-alerts-link"
                >
                  Alert rules →
                </Link>
                <Link
                  to={pathWithQuery(viewToPath('workloads'), {
                    workload: selected.workload.name,
                    tab: 'trust',
                  })}
                  className="text-xs text-brand hover:underline"
                  data-testid="health-trust-link"
                >
                  Trust & attestation →
                </Link>
                <Link
                  to={viewToPath('sla')}
                  className="text-xs text-brand hover:underline"
                  data-testid="health-sla-link"
                >
                  SLA compliance →
                </Link>
                <span>
                  Last restart count: <span className="text-foreground">{selected.history.last_restart_count}</span>
                </span>
                <span>
                  Current restarts: <span className="text-foreground">{selected.workload.restart_count}</span>
                </span>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default withAuroraPage('health', HealthPage);