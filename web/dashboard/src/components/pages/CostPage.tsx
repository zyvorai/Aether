import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState } from 'react';
import { useNavigate, Link } from 'react-router';
import { Inbox } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatUSD } from '../../utils/formatters';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { useApiPage } from '../../hooks/useApiPage';
import CostIntelligencePanel from '../CostIntelligencePanel';
import FinOpsPlatformPanel from '../FinOpsPlatformPanel';
import EmptyState from '../EmptyState';
import InlineActionError from '../InlineActionError';
import PanelLoadError from '../PanelLoadError';
import SpecWorkbench from '../SpecWorkbench';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { CostEstimate } from '../../types/api';

interface ChargebackReport {
  totalMonthlyUsd: number;
  totalSpotMonthlyUsd: number;
  tco36MonthsUsd: number;
  pricingSource: string;
  region: string;
  lines: { workload: string; owner: string; project: string; monthlyUsd: number }[];
}

function CostPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [workloadQuery] = useQueryParam('workload');
  const [estimates, setEstimates] = useState<CostEstimate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const {
    data: chargeback,
    loading: chargebackLoading,
    error: chargebackError,
    reload: reloadChargeback,
  } = useApiPage<ChargebackReport>(
    () => apiFetchSettled<ChargebackReport>('/cost/chargeback?provider=aws'),
    [refreshKey],
  );

  async function handleEstimate(yaml: string) {
    setLoading(true);
    setError(null);
    const res = await apiPost<CostEstimate[]>('/cost', { yaml });
    if (res.success && res.data) {
      setEstimates(res.data);
    } else {
      setError(res.error ?? 'Failed to estimate costs');
      setEstimates([]);
    }
    setLoading(false);
  }

  const cheapest = estimates.length > 0
    ? estimates.reduce((a, b) => (a.total_monthly < b.total_monthly ? a : b)).provider
    : null;

  const resultContent =
    estimates.length === 0 ? (
      error ? (
        <InlineActionError message={error} />
      ) : (
        <EmptyState icon={<Inbox size={48} />} title="No estimates" description="Submit a workload YAML to see cost estimates" />
      )
    ) : (
      <div className="space-y-4" data-testid="cost-estimate-results">
        {estimates.map((est) => (
          <div
            key={est.provider}
            className={`rounded-xl border p-4 ${
              est.provider === cheapest
                ? 'border-emerald-500/30 bg-emerald-500/5'
                : 'glass-divider glass'
            }`}
          >
            <div className="flex items-center justify-between gap-2 mb-3">
              <span className="font-medium text-foreground">{est.provider}</span>
              {est.provider === cheapest && (
                <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                  Cheapest
                </span>
              )}
            </div>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between gap-4">
                <dt className="text-subtle">CPU / month</dt>
                <dd className="text-foreground">{formatUSD(est.cpu_cost_monthly)}</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-subtle">Memory / month</dt>
                <dd className="text-foreground">{formatUSD(est.memory_cost_monthly)}</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-subtle">Storage / month</dt>
                <dd className="text-foreground">{formatUSD(est.storage_cost_monthly)}</dd>
              </div>
              <div className="flex justify-between gap-4 font-medium">
                <dt className="text-muted">Total / month</dt>
                <dd className={est.provider === cheapest ? 'text-emerald-400' : 'text-foreground'}>
                  {formatUSD(est.total_monthly)}
                </dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-subtle">Hourly</dt>
                <dd className="text-muted">{formatUSD(est.total_hourly)}</dd>
              </div>
            </dl>
          </div>
        ))}
      </div>
    );

  const hubBanner = (
    <div className="mb-6 glass-context-banner" data-testid="cost-hub-context">
      FinOps
      {' · '}
      <Link to={viewToPath('intelligence')} className="text-brand hover:underline" data-testid="cost-context-intelligence-hub-link">
        Intelligence →
      </Link>
      {' · '}
      <Link to={viewToPath('hosted')} className="text-brand hover:underline" data-testid="cost-context-hosted-link">
        Hosted SaaS →
      </Link>
      {' · '}
      <Link to={viewToPath('migrations')} className="text-brand hover:underline" data-testid="cost-context-migrations-link">
        Migrations →
      </Link>
      {' · '}
      <Link to={viewToPath('platform')} className="text-brand hover:underline" data-testid="cost-context-platform-hub-link">
        Platform →
      </Link>
    </div>
  );

  return (
    <div className="space-y-6">
      {hubBanner}
      <CostIntelligencePanel />
      <FinOpsPlatformPanel />
      {workloadQuery.trim() ? (
        <WorkloadContextBanner
          testId="cost-workload-context"
          workload={workloadQuery}
          openTestId="cost-open-workload"
          description="Cost context"
        >
          <WorkloadScopedCrossLinks workload={workloadQuery} prefix="cost" showMetrics />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('scheduler'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="cost-scheduler-scoped-link"
          >
            Scheduler →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('intelligence'), { workload: workloadQuery.trim(), tab: 'cost' })}
            className="text-brand hover:underline"
            data-testid="cost-intelligence-link"
          >
            Intelligence →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('affinity'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="cost-context-affinity-link"
          >
            Affinity →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="cost-context-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('drift'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="cost-context-drift-link"
          >
            Drift →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadQuery.trim() })}
            className="text-brand hover:underline"
            data-testid="cost-context-secrets-link"
          >
            Secrets →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <section className="glass mb-6 space-y-6 p-6 sm:p-8">
      {chargebackLoading ? null : chargebackError ? (
        <PanelLoadError
          title="Fleet chargeback unavailable"
          description="Could not load chargeback data from the API."
          onRetry={() => void reloadChargeback()}
        />
      ) : chargeback ? (
        <div className="glass" data-testid="cost-fleet-chargeback">
          <div className="flex flex-wrap items-center justify-between gap-3 mb-2">
            <h2 className="text-lg font-semibold text-foreground">Fleet chargeback</h2>
            <button
              type="button"
              data-testid="cost-fleet-metrics-link"
              onClick={() =>
                navigate(
                  workloadQuery.trim()
                    ? pathWithQuery(viewToPath('metrics'), { workload: workloadQuery.trim() })
                    : viewToPath('metrics'),
                )
              }
              className="text-xs text-brand hover:underline"
            >
              Full report on Metrics →
            </button>
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { tab: 'cost' })}
              className="text-xs text-brand hover:underline"
            >
              Cost optimize →
            </Link>
            <Link
              to={viewToPath('scheduler')}
              className="text-xs text-brand hover:underline"
              data-testid="cost-scheduler-link"
            >
              Placement scheduler →
            </Link>
          </div>
          <p className="text-sm text-muted">
            {chargeback.pricingSource} · {chargeback.region} · fleet {formatUSD(chargeback.totalMonthlyUsd)}/mo
            · {chargeback.lines.length} workload line(s)
          </p>
          {chargeback.lines.length > 0 && (
            <ul className="mt-3 space-y-1 text-sm">
              {chargeback.lines.map((line) => (
                <li
                  key={line.workload}
                  className={`flex flex-wrap items-center justify-between gap-2 ${workloadQuery.trim() === line.workload ? 'rounded-lg border border-brand/40 bg-brand/5 px-2 py-1' : ''}`}
                  data-testid={workloadQuery.trim() === line.workload ? 'cost-workload-highlight' : undefined}
                >
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: line.workload })}
                    className="text-brand hover:underline font-mono text-xs"
                  >
                    {line.workload}
                  </Link>
                  <span className="text-subtle">{formatUSD(line.monthlyUsd)}/mo</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}

      <div data-testid="cost-estimate-form">
      <SpecWorkbench
        title="Cost estimation"
        description="Paste workload YAML to compare provider pricing."
        buttonText={loading ? 'Estimating…' : 'Estimate Costs'}
        onSubmit={handleEstimate}
        loading={loading}
        placeholder="Paste workload YAML to estimate costs..."
        submitTestId="cost-estimate-submit"
        result={resultContent}
      />
      </div>
      </section>
    </div>
  );
}

export default withAuroraPage('cost', CostPage);
