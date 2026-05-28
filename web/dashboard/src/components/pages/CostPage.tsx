// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { useNavigate, Link } from 'react-router';
import { Inbox } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatUSD } from '../../utils/formatters';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import EmptyState from '../EmptyState';
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

export default function CostPage() {
  const navigate = useNavigate();
  const [workloadQuery] = useQueryParam('workload');
  const [estimates, setEstimates] = useState<CostEstimate[]>([]);
  const [chargeback, setChargeback] = useState<ChargebackReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadChargeback = useCallback(async () => {
    const res = await apiFetchSettled<ChargebackReport>('/cost/chargeback?provider=aws');
    setChargeback(res.ok ? res.data : null);
  }, []);

  useEffect(() => {
    void loadChargeback();
  }, [loadChargeback]);

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
        <p className="text-sm text-red-400">{error}</p>
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
                : 'border-slate-800/80 bg-slate-950/40'
            }`}
          >
            <div className="flex items-center justify-between gap-2 mb-3">
              <span className="font-medium text-slate-100">{est.provider}</span>
              {est.provider === cheapest && (
                <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                  Cheapest
                </span>
              )}
            </div>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between gap-4">
                <dt className="text-slate-500">CPU / month</dt>
                <dd className="text-slate-200">{formatUSD(est.cpu_cost_monthly)}</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-slate-500">Memory / month</dt>
                <dd className="text-slate-200">{formatUSD(est.memory_cost_monthly)}</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-slate-500">Storage / month</dt>
                <dd className="text-slate-200">{formatUSD(est.storage_cost_monthly)}</dd>
              </div>
              <div className="flex justify-between gap-4 font-medium">
                <dt className="text-slate-400">Total / month</dt>
                <dd className={est.provider === cheapest ? 'text-emerald-400' : 'text-slate-100'}>
                  {formatUSD(est.total_monthly)}
                </dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-slate-500">Hourly</dt>
                <dd className="text-slate-400">{formatUSD(est.total_hourly)}</dd>
              </div>
            </dl>
          </div>
        ))}
      </div>
    );

  return (
    <div className="space-y-6">
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
            className="text-aether hover:underline"
            data-testid="cost-scheduler-scoped-link"
          >
            Scheduler →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('intelligence'), { workload: workloadQuery.trim(), tab: 'cost' })}
            className="text-aether hover:underline"
            data-testid="cost-intelligence-link"
          >
            Intelligence →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      {chargeback ? (
        <div className="dash-card" data-testid="cost-fleet-chargeback">
          <div className="flex flex-wrap items-center justify-between gap-3 mb-2">
            <h2 className="text-lg font-semibold text-zinc-100">Fleet chargeback</h2>
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
              className="text-xs text-aether hover:underline"
            >
              Full report on Metrics →
            </button>
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { tab: 'cost' })}
              className="text-xs text-aether hover:underline"
            >
              Cost optimize →
            </Link>
            <Link
              to={viewToPath('scheduler')}
              className="text-xs text-aether hover:underline"
              data-testid="cost-scheduler-link"
            >
              Placement scheduler →
            </Link>
          </div>
          <p className="text-sm text-zinc-400">
            {chargeback.pricingSource} · {chargeback.region} · fleet {formatUSD(chargeback.totalMonthlyUsd)}/mo
            · {chargeback.lines.length} workload line(s)
          </p>
          {chargeback.lines.length > 0 && (
            <ul className="mt-3 space-y-1 text-sm">
              {chargeback.lines.map((line) => (
                <li
                  key={line.workload}
                  className={`flex flex-wrap items-center justify-between gap-2 ${workloadQuery.trim() === line.workload ? 'rounded-lg border border-aether/40 bg-aether/5 px-2 py-1' : ''}`}
                  data-testid={workloadQuery.trim() === line.workload ? 'cost-workload-highlight' : undefined}
                >
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: line.workload })}
                    className="text-aether hover:underline font-mono text-xs"
                  >
                    {line.workload}
                  </Link>
                  <span className="text-zinc-500">{formatUSD(line.monthlyUsd)}/mo</span>
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
        buttonText="Estimate Costs"
        onSubmit={handleEstimate}
        loading={loading}
        placeholder="Paste workload YAML to estimate costs..."
        submitTestId="cost-estimate-submit"
        result={resultContent}
      />
      </div>
    </div>
  );
}
