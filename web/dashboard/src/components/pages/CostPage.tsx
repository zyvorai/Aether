// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState } from 'react';
import { Inbox } from 'lucide-react';
import { apiPost } from '../../utils/api';
import { formatUSD } from '../../utils/formatters';
import EmptyState from '../EmptyState';
import SpecWorkbench from '../SpecWorkbench';
import type { CostEstimate } from '../../types/api';

export default function CostPage() {
  const [estimates, setEstimates] = useState<CostEstimate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      <div className="space-y-4">
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
    <SpecWorkbench
      title="Cost estimation"
      description="Paste workload YAML to compare provider pricing."
      buttonText="Estimate Costs"
      onSubmit={handleEstimate}
      loading={loading}
      placeholder="Paste workload YAML to estimate costs..."
      result={resultContent}
    />
  );
}
