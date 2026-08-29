// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { AlertTriangle, ArrowRight, CheckCircle2, Clock, Loader2, Rocket, Shield } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { formatPercent, formatUSD } from '../utils/formatters';
import type { MigrationAdvice, MigrationPlanProposal, WorkloadResponse } from '../types/api';
import WorkloadSelect from './WorkloadSelect';
import Badge from './Badge';
import GlassSection from './GlassSection';

const TARGET_RUNTIMES = ['kube', 'podman', 'kubevirt', 'metal3'] as const;

interface RiskRow {
  label: string;
  level: string;
  detail: string;
}

function riskTone(level: string): 'green' | 'yellow' | 'red' | 'muted' {
  const l = level.toLowerCase();
  if (l === 'low') return 'green';
  if (l === 'medium' || l === 'moderate') return 'yellow';
  if (l === 'high' || l === 'critical') return 'red';
  return 'muted';
}

function deriveRiskMatrix(advice: MigrationAdvice, plan: MigrationPlanProposal): RiskRow[] {
  const base = advice.risk_level;
  const warnings = advice.warnings.join(' ').toLowerCase();
  const reasons = advice.reasons.join(' ').toLowerCase();

  const storageLevel =
    warnings.includes('storage') || warnings.includes('persistent') || reasons.includes('pvc')
      ? base === 'Low'
        ? 'Medium'
        : base
      : 'Low';
  const networkLevel = warnings.includes('network') || warnings.includes('ingress') ? 'Medium' : 'Low';
  const dnsLevel = warnings.includes('dns') ? 'Medium' : 'Low';
  const gpuLevel =
    warnings.includes('gpu') || reasons.includes('gpu')
      ? base === 'Low'
        ? 'High'
        : 'High'
      : 'Low';

  return [
    { label: 'Storage migration', level: storageLevel, detail: 'Volume and PVC portability' },
    { label: 'Network', level: networkLevel, detail: 'Service, ingress, and policy cutover' },
    { label: 'DNS', level: dnsLevel, detail: 'Endpoint and name resolution' },
    { label: 'GPU compatibility', level: gpuLevel, detail: 'Accelerator availability on target' },
  ];
}

function normalizePlan(
  raw: MigrationPlanProposal,
  workloadName: string,
  sourceRuntime: string,
  targetRuntime: string,
): MigrationPlanProposal {
  const advice = raw.advice;
  return {
    ...raw,
    advice: {
      workload_name: advice.workload_name || workloadName,
      source_runtime: advice.source_runtime || sourceRuntime,
      target_runtime: advice.target_runtime || targetRuntime,
      recommended_strategy: String(advice.recommended_strategy ?? 'BlueGreen'),
      estimated_downtime_secs: advice.estimated_downtime_secs ?? 0,
      risk_level: String(advice.risk_level ?? 'Medium'),
      reasons: advice.reasons ?? [],
      warnings: advice.warnings ?? [],
      timing: advice.timing,
      canary_config: advice.canary_config,
    },
  };
}

export default function MigrationPlannerPanel() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [workload, setWorkload] = useState('');
  const [target, setTarget] = useState<string | null>(null);
  const [plan, setPlan] = useState<MigrationPlanProposal | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<WorkloadResponse[]>('/workloads').then((data) =>
      // Migration planning only works for Aether-managed workloads (they have a stored
      // spec to plan against); discovered cluster workloads have no spec_path and their
      // slash-qualified "cluster/namespace/name" identifiers aren't valid workload names,
      // so offering them here would always fail with "Could not load migration plan".
      setWorkloads((data ?? []).filter((w) => w.source !== 'cluster')),
    );
  }, []);

  const selected = useMemo(
    () => workloads.find((w) => w.name === workload) ?? null,
    [workloads, workload],
  );

  const loadPlan = useCallback(
    async (name: string, targetRuntime: string) => {
      const ws = workloads.find((w) => w.name === name);
      setLoading(true);
      setError(null);
      setTarget(targetRuntime);
      const data = await apiFetch<MigrationPlanProposal>(
        `/ai/migration-plan/${encodeURIComponent(name)}/${encodeURIComponent(targetRuntime)}`,
      );
      setLoading(false);
      if (!data) {
        setPlan(null);
        setError('Could not load migration plan');
        return;
      }
      setPlan(normalizePlan(data, name, ws?.runtime ?? '', targetRuntime));
    },
    [workloads],
  );

  const riskRows = plan ? deriveRiskMatrix(plan.advice, plan) : [];
  const confidence = plan ? Math.round((1 - plan.rollback_probability) * 100) : 0;

  return (
    <GlassSection
      accent="blue"
      testId="migration-planner"
      variant="hero"
      label="AI Migration Planner"
      title="Move workload with risk analysis"
      subtitle="Select a workload and target runtime. Aether recommends strategy, predicted downtime, and migration risk."
      icon={<Rocket className="h-5 w-5 text-brand" />}
    >
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
        <div className="space-y-4">
          <div>
            <label className="mb-2 block text-xs font-medium uppercase tracking-wider text-ink-3">
              Workload
            </label>
            <WorkloadSelect
              workloads={workloads.map((w) => w.name)}
              value={workload}
              onChange={setWorkload}
              placeholder="Select workload to migrate"
            />
            {workloads.length === 0 ? (
              <p className="mt-2 text-xs text-ink-3">
                No Aether-managed workloads yet. Deploy one to plan a migration — discovered
                cluster workloads aren't manageable by Aether until deployed through it.
              </p>
            ) : null}
          </div>

          {selected ? (
            <div className="glass p-4 text-sm">
              <div className="text-ink-2">Current runtime</div>
              <div className="mt-1 font-medium capitalize text-ink">{selected.runtime}</div>
            </div>
          ) : null}

          <div>
            <span className="mb-2 block text-xs font-medium uppercase tracking-wider text-ink-3">
              Target runtime
            </span>
            <div className="grid grid-cols-2 gap-2">
              {TARGET_RUNTIMES.map((rt) => (
                <button
                  key={rt}
                  type="button"
                  disabled={!workload || loading || selected?.runtime === rt}
                  onClick={() => workload && void loadPlan(workload, rt)}
                  className={`rounded-xl border px-3 py-2.5 text-sm font-medium capitalize transition ${
                    target === rt
                      ? 'border-brand/50 bg-brand/10 text-brand'
                      : 'glass-divider glass text-ink-2 hover:border-brand/30'
                  } disabled:opacity-40`}
                  data-testid={`migration-target-${rt}`}
                >
                  {rt}
                </button>
              ))}
            </div>
          </div>

          {loading ? (
            <div className="flex items-center gap-2 text-sm text-ink-3">
              <Loader2 className="h-4 w-4 animate-spin" />
              Analyzing migration path…
            </div>
          ) : null}
          {error ? <p className="text-sm text-red-400">{error}</p> : null}
        </div>

        {plan ? (
          <div className="space-y-4" data-testid="migration-plan-result">
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
                <div className="text-[10px] uppercase tracking-wider text-ink-3">Recommended strategy</div>
                <div className="mt-2 text-lg font-semibold capitalize text-ink">
                  {String(plan.advice.recommended_strategy).replace(/-/g, ' ')}
                </div>
              </div>
              <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
                <div className="text-[10px] uppercase tracking-wider text-ink-3">Predicted downtime</div>
                <div className="mt-2 text-lg font-semibold text-ink">
                  {plan.advice.estimated_downtime_secs}s
                </div>
              </div>
              <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
                <div className="text-[10px] uppercase tracking-wider text-ink-3">Confidence</div>
                <div className="mt-2 text-lg font-semibold text-emerald-300">{confidence}%</div>
              </div>
              <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
                <div className="text-[10px] uppercase tracking-wider text-ink-3">Cost impact</div>
                <div className="mt-2 text-lg font-semibold text-ink">{formatUSD(plan.cost_impact_usd)}/mo</div>
              </div>
            </div>

            <div className="glass p-4">
              <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-ink">
                <Shield className="h-4 w-4 text-brand" />
                Risk analysis
              </div>
              <div className="space-y-2">
                {riskRows.map((row) => (
                  <div
                    key={row.label}
                    className="flex items-center justify-between gap-3 rounded-xl border glass-divider px-3 py-2"
                  >
                    <div>
                      <div className="text-sm text-ink">{row.label}</div>
                      <div className="text-xs text-ink-3">{row.detail}</div>
                    </div>
                    <Badge text={row.level} variant={riskTone(row.level)} />
                  </div>
                ))}
              </div>
            </div>

            {plan.advice.reasons.length > 0 ? (
              <div className="rounded-2xl border border-emerald-500/20 bg-emerald-500/5 p-4">
                <div className="mb-2 flex items-center gap-2 text-sm font-medium text-emerald-200">
                  <CheckCircle2 className="h-4 w-4" />
                  Reasons
                </div>
                <ul className="space-y-1 text-sm text-emerald-100/90">
                  {plan.advice.reasons.map((r) => (
                    <li key={r}>✓ {r}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            {plan.advice.warnings.length > 0 ? (
              <div className="rounded-2xl border border-amber-500/20 bg-amber-500/5 p-4">
                <div className="mb-2 flex items-center gap-2 text-sm font-medium text-amber-200">
                  <AlertTriangle className="h-4 w-4" />
                  Warnings
                </div>
                <ul className="space-y-1 text-sm text-amber-100/90">
                  {plan.advice.warnings.map((w) => (
                    <li key={w}>{w}</li>
                  ))}
                </ul>
              </div>
            ) : null}

            <div className="flex flex-wrap items-center gap-3 text-xs text-ink-3">
              <span className="inline-flex items-center gap-1">
                <Clock className="h-3.5 w-3.5" />
                ETA {plan.eta_secs}s
              </span>
              <span>Blast radius {formatPercent(plan.blast_radius_score, 0)}</span>
              <span>Rollback risk {formatPercent(plan.rollback_probability, 0)}</span>
              {plan.auto_eligible ? (
                <span className="inline-flex items-center gap-1 text-emerald-400">
                  <ArrowRight className="h-3.5 w-3.5" />
                  Auto-eligible
                </span>
              ) : null}
            </div>
          </div>
        ) : (
          <div className="glass-empty-state min-h-[280px] border-dashed p-8 text-center">
            <p className="max-w-sm text-sm text-ink-3">
              Choose a workload and target runtime to generate a migration plan with risk matrix and strategy.
            </p>
          </div>
        )}
      </div>
    </GlassSection>
  );
}
