// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { DollarSign, Leaf, Loader2, RefreshCw, TrendingUp } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { formatPercent, formatUSD } from '../utils/formatters';
import GlassSection from './GlassSection';

interface ChargebackAutomation {
  total_monthly_usd: number;
  unassigned_count: number;
  by_owner: Record<string, number>;
}

interface SpotAdvisor {
  eligible_count: number;
  entries: Array<{ workload: string; eligible: boolean; savings_pct: number; reason: string }>;
}

interface ReservedPlanner {
  fleet_reserved_savings_usd: number;
  recommendations: Array<{ workload: string; savings_pct: number; recommendation: string }>;
}

interface CostAnomalies {
  alert: boolean;
  fleet_delta_pct: number;
  anomalies: Array<{ workload: string; delta_pct: number; severity: string }>;
}

interface UnitEconomics {
  fleet_cost_per_1k_usd: number;
  entries: Array<{ workload: string; cost_per_1k_requests_usd: number }>;
}

interface MulticloudCompare {
  recommended_provider: string;
  savings_vs_worst_pct: number;
  rows: Array<{ provider: string; total_monthly_usd: number }>;
}

interface FinOpsTrends {
  trend_direction: string;
  forecast_next_month_usd: number;
  points: Array<{ label: string; monthly_usd: number; forecast?: boolean }>;
}

interface BudgetWebhook {
  breached: boolean;
  message: string;
  cap_usd: number;
  current_usd: number;
}

export default function FinOpsPlatformPanel() {
  const [tab, setTab] = useState<
    'chargeback' | 'spot' | 'reserved' | 'anomalies' | 'unit' | 'multicloud' | 'carbon' | 'trends'
  >('trends');
  const [loading, setLoading] = useState(true);
  const [chargeback, setChargeback] = useState<ChargebackAutomation | null>(null);
  const [spot, setSpot] = useState<SpotAdvisor | null>(null);
  const [reserved, setReserved] = useState<ReservedPlanner | null>(null);
  const [anomalies, setAnomalies] = useState<CostAnomalies | null>(null);
  const [unit, setUnit] = useState<UnitEconomics | null>(null);
  const [multicloud, setMulticloud] = useState<MulticloudCompare | null>(null);
  const [carbon, setCarbon] = useState<{ hint: string; entries: Array<{ carbon_kg_monthly: number; region: string }> } | null>(null);
  const [trends, setTrends] = useState<FinOpsTrends | null>(null);
  const [budget, setBudget] = useState<BudgetWebhook | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [cb, sp, ri, an, ue, mc, cbn, tr] = await Promise.all([
      apiFetch<ChargebackAutomation>('/intelligence/finops/chargeback'),
      apiFetch<SpotAdvisor>('/intelligence/finops/spot-advisor'),
      apiFetch<ReservedPlanner>('/intelligence/finops/reserved-planner'),
      apiFetch<CostAnomalies>('/intelligence/finops/anomalies'),
      apiFetch<UnitEconomics>('/intelligence/finops/unit-economics'),
      apiFetch<MulticloudCompare>('/intelligence/finops/multicloud-compare'),
      apiFetch<{ hint: string; entries: Array<{ carbon_kg_monthly: number; region: string }> }>('/intelligence/finops/carbon'),
      apiFetch<FinOpsTrends>('/intelligence/finops/trends'),
    ]);
    setChargeback(cb);
    setSpot(sp);
    setReserved(ri);
    setAnomalies(an);
    setUnit(ue);
    setMulticloud(mc);
    setCarbon(cbn);
    setTrends(tr);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function checkBudget() {
    const res = await apiPost<BudgetWebhook>('/intelligence/finops/budget-webhook', {});
    if (res.success) setBudget(res.data ?? null);
  }

  async function executeFinOps(dryRun: boolean) {
    const res = await apiPost<{ applied: string[]; savings_monthly_usd: number }>(
      '/intelligence/finops/execute',
      { dry_run: dryRun, schedule: 'nightly-02:00' },
    );
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `FinOps dry-run: ${res.data?.applied.length ?? 0} actions`
              : `FinOps execute: ${formatUSD(res.data?.savings_monthly_usd ?? 0)}/mo savings`,
            type: 'success',
          },
        }),
      );
    }
  }

  const tabs = [
    { id: 'trends' as const, label: 'Trends' },
    { id: 'chargeback' as const, label: 'Chargeback' },
    { id: 'spot' as const, label: 'Spot' },
    { id: 'reserved' as const, label: 'RI Planner' },
    { id: 'anomalies' as const, label: 'Anomalies' },
    { id: 'unit' as const, label: 'Unit $' },
    { id: 'multicloud' as const, label: 'Multi-cloud' },
    { id: 'carbon' as const, label: 'Carbon' },
  ];

  return (
    <GlassSection
      accent="blue"
      testId="finops-platform-panel"
      title="FinOps Platform v2"
      subtitle="Chargeback, spot/RI planning, anomalies, unit economics, multi-cloud compare, trends"
      icon={<DollarSign className="h-5 w-5 text-emerald-400" />}
      actions={
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => void executeFinOps(true)}
            className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-200"
            data-testid="finops-execute-dry-run"
          >
            Agent dry-run
          </button>
          <button
            type="button"
            onClick={() => void checkBudget()}
            className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-200"
            data-testid="finops-budget-webhook"
          >
            Budget check
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
      }
    >
      {budget ? (
        <p
          className={`mb-4 rounded-lg border px-3 py-2 text-xs ${budget.breached ? 'border-amber-500/40 text-amber-200' : 'border-slate-700 text-slate-400'}`}
          data-testid="finops-budget-status"
        >
          {budget.message}
        </p>
      ) : null}

      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-200' : 'border-slate-700 text-slate-400'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'trends' ? (
        <div data-testid="finops-trends-panel" className="space-y-3">
          <div className="flex items-center gap-2 text-sm text-slate-300">
            <TrendingUp className="h-4 w-4 text-emerald-400" />
            Forecast: {formatUSD(trends?.forecast_next_month_usd ?? 0)}/mo · trend {trends?.trend_direction ?? '—'}
          </div>
          <div className="flex h-24 items-end gap-1">
            {(trends?.points ?? []).map((p) => (
              <div
                key={p.label}
                className={`flex-1 rounded-t ${p.forecast ? 'bg-emerald-500/30' : 'bg-emerald-500/60'}`}
                style={{ height: `${Math.max(8, Math.min(100, p.monthly_usd / 50))}%` }}
                title={`${p.label}: ${formatUSD(p.monthly_usd)}`}
              />
            ))}
          </div>
        </div>
      ) : null}

      {tab === 'chargeback' ? (
        <div data-testid="finops-chargeback-panel" className="text-sm text-slate-300">
          <p>Fleet total: {formatUSD(chargeback?.total_monthly_usd ?? 0)}/mo</p>
          <p className="text-xs text-slate-500">Unassigned: {chargeback?.unassigned_count ?? 0}</p>
          <ul className="mt-2 space-y-1 text-xs text-slate-400">
            {Object.entries(chargeback?.by_owner ?? {})
              .slice(0, 6)
              .map(([owner, usd]) => (
                <li key={owner} className="flex justify-between">
                  <span>{owner}</span>
                  <span>{formatUSD(usd)}</span>
                </li>
              ))}
          </ul>
        </div>
      ) : null}

      {tab === 'spot' ? (
        <div data-testid="finops-spot-panel">
          <p className="mb-2 text-sm text-emerald-300">{spot?.eligible_count ?? 0} spot-eligible workload(s)</p>
          <ul className="space-y-2 text-xs text-slate-400">
            {(spot?.entries ?? []).slice(0, 5).map((e) => (
              <li key={e.workload} className="rounded border border-slate-800 px-2 py-1">
                <span className="font-mono text-slate-200">{e.workload}</span>
                {e.eligible ? ` · save ${formatPercent(e.savings_pct, 0)}` : ' · not eligible'} — {e.reason}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'reserved' ? (
        <div data-testid="finops-reserved-panel" className="text-sm">
          <p className="text-emerald-300">Fleet RI savings potential: {formatUSD(reserved?.fleet_reserved_savings_usd ?? 0)}/mo</p>
          <ul className="mt-2 space-y-1 text-xs text-slate-400">
            {(reserved?.recommendations ?? []).slice(0, 4).map((r) => (
              <li key={r.workload}>{r.recommendation}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'anomalies' ? (
        <div data-testid="finops-anomalies-panel">
          {anomalies?.alert ? (
            <p className="text-sm text-amber-300">Spend alert: fleet {formatPercent(anomalies.fleet_delta_pct, 1)} vs baseline</p>
          ) : (
            <p className="text-sm text-slate-500">No cost anomalies detected.</p>
          )}
        </div>
      ) : null}

      {tab === 'unit' ? (
        <div data-testid="finops-unit-panel" className="text-sm text-slate-300">
          Fleet: {formatUSD(unit?.fleet_cost_per_1k_usd ?? 0)} per 1k requests
        </div>
      ) : null}

      {tab === 'multicloud' ? (
        <div data-testid="finops-multicloud-panel" className="space-y-2 text-sm">
          <p className="text-emerald-300">
            Recommended: {multicloud?.recommended_provider ?? '—'} (save {formatPercent(multicloud?.savings_vs_worst_pct ?? 0, 0)} vs worst)
          </p>
          {(multicloud?.rows ?? []).map((r) => (
            <div key={r.provider} className="flex justify-between text-xs text-slate-400">
              <span>{r.provider}</span>
              <span>{formatUSD(r.total_monthly_usd)}</span>
            </div>
          ))}
        </div>
      ) : null}

      {tab === 'carbon' ? (
        <div data-testid="finops-carbon-panel" className="flex items-start gap-2 text-sm text-slate-400">
          <Leaf className="mt-0.5 h-4 w-4 text-emerald-500" />
          <div>
            <p>{carbon?.entries?.[0] ? `${carbon.entries[0].carbon_kg_monthly.toFixed(1)} kg CO₂/mo (${carbon.entries[0].region})` : '—'}</p>
            <p className="text-xs">{carbon?.hint}</p>
          </div>
        </div>
      ) : null}
    </GlassSection>
  );
}
