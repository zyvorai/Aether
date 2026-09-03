// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { FlaskConical, Loader2, Play, RefreshCw } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { formatPercent, formatUSD } from '../utils/formatters';
import type { TwinSimulateReport, WorkloadResponse } from '../types/api';
import GlassSection from './GlassSection';

export default function DigitalTwinPanel() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [workload, setWorkload] = useState('');
  const [scale, setScale] = useState(1.5);
  const [targetRuntime, setTargetRuntime] = useState('');
  const [report, setReport] = useState<TwinSimulateReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [booting, setBooting] = useState(true);

  useEffect(() => {
    apiFetch<WorkloadResponse[]>('/workloads').then((data) => {
      setWorkloads(data ?? []);
      setBooting(false);
    });
  }, []);

  const runSimulation = useCallback(async () => {
    setLoading(true);
    const res = await apiPost<TwinSimulateReport>('/intelligence/digital-twin/simulate', {
      workload: workload.trim() || undefined,
      scale_factor: scale,
      target_runtime: targetRuntime.trim() || undefined,
    });
    setReport(res.success ? res.data ?? null : null);
    setLoading(false);
  }, [workload, scale, targetRuntime]);

  useEffect(() => {
    if (!booting) void runSimulation();
  }, [booting, runSimulation]);

  return (
    <GlassSection
      accent="blue"
      testId="digital-twin-panel"
      title="Digital Twin"
      subtitle="What-if simulation — project capacity, cost, and risk before you change production."
      icon={<FlaskConical className="h-5 w-5 text-lavender" />}
      actions={
        <button
          type="button"
          onClick={() => void runSimulation()}
          disabled={loading}
          className="inline-flex items-center gap-2 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-white hover:bg-primary/90 disabled:opacity-60"
        >
          {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
          Run simulation
        </button>
      }
    >
      <div className="mb-6 grid gap-4 md:grid-cols-3">
        <label className="block text-sm">
          <span className="mb-1 block text-xs text-subtle">Scope</span>
          <select
            value={workload}
            onChange={(e) => setWorkload(e.target.value)}
            className="glass-input"
          >
            <option value="">Entire fleet</option>
            {workloads.map((w) => (
              <option key={w.name} value={w.name}>
                {w.name}
              </option>
            ))}
          </select>
        </label>
        <label className="block text-sm">
          <span className="mb-1 block text-xs text-subtle">Scale factor ({scale.toFixed(1)}×)</span>
          <input
            type="range"
            min={1}
            max={3}
            step={0.1}
            value={scale}
            onChange={(e) => setScale(Number(e.target.value))}
            className="w-full"
          />
        </label>
        <label className="block text-sm">
          <span className="mb-1 block text-xs text-subtle">Target runtime (optional)</span>
          <select
            value={targetRuntime}
            onChange={(e) => setTargetRuntime(e.target.value)}
            className="glass-input"
          >
            <option value="">No change</option>
            <option value="kubernetes">kubernetes</option>
            <option value="kubevirt">kubevirt</option>
            <option value="metal3">metal3</option>
            <option value="podman">podman</option>
          </select>
        </label>
      </div>

      {report ? (
        <div className="space-y-4">
          <p className="text-sm text-lavender/90">Scenario: {report.scenario}</p>
          <div className="grid gap-4 lg:grid-cols-2">
            <TwinSnapshotCard title="Baseline" snapshot={report.baseline} />
            <TwinSnapshotCard title="Projected" snapshot={report.projected} highlight />
          </div>
          <div className="grid gap-3 sm:grid-cols-4">
            <DeltaCard label="Risk Δ" value={formatPercent(report.deltas.risk_delta, 1)} />
            <DeltaCard label="CPU Δ" value={formatPercent(report.deltas.cpu_util_delta, 1)} />
            <DeltaCard label="Memory Δ" value={formatPercent(report.deltas.memory_util_delta, 1)} />
            <DeltaCard label="Cost Δ" value={formatUSD(report.deltas.cost_delta_usd)} />
          </div>
          <ul className="space-y-2">
            {report.recommendations.map((line) => (
              <li key={line} className="rounded-lg border glass-divider px-3 py-2 text-sm text-muted">
                {line}
              </li>
            ))}
          </ul>
        </div>
      ) : loading ? (
        <div className="flex items-center gap-2 text-sm text-subtle">
          <Loader2 className="h-4 w-4 animate-spin" />
          Simulating…
        </div>
      ) : null}
    </GlassSection>
  );
}

function TwinSnapshotCard({
  title,
  snapshot,
  highlight = false,
}: {
  title: string;
  snapshot: TwinSimulateReport['baseline'];
  highlight?: boolean;
}) {
  return (
    <div
      className={`rounded-2xl border p-4 ${highlight ? 'border-lavender/30 bg-lavender/5' : 'glass-divider glass'}`}
    >
      <p className="mb-3 text-xs font-semibold uppercase tracking-wider text-subtle">{title}</p>
      <dl className="grid grid-cols-2 gap-3 text-sm">
        <div>
          <dt className="text-subtle">Risk</dt>
          <dd className="text-foreground">{formatPercent(snapshot.fleet_risk_score, 0)}</dd>
        </div>
        <div>
          <dt className="text-subtle">Saturation</dt>
          <dd className="text-foreground">{snapshot.saturation_days}d</dd>
        </div>
        <div>
          <dt className="text-subtle">CPU util</dt>
          <dd className="text-foreground">{formatPercent(snapshot.avg_cpu_utilization, 0)}</dd>
        </div>
        <div>
          <dt className="text-subtle">Memory util</dt>
          <dd className="text-foreground">{formatPercent(snapshot.avg_memory_utilization, 0)}</dd>
        </div>
        <div className="col-span-2">
          <dt className="text-subtle">Est. monthly cost</dt>
          <dd className="text-foreground">{formatUSD(snapshot.estimated_monthly_cost_usd)}</dd>
        </div>
      </dl>
    </div>
  );
}

function DeltaCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="glass px-3 py-2">
      <div className="text-xs text-subtle">{label}</div>
      <div className="text-lg font-semibold text-foreground">{value}</div>
    </div>
  );
}
