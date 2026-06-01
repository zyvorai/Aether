// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Gauge, Loader2, Play, RefreshCw } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import Badge from './Badge';
import type { CapacityScaleExecuteReport, CapacityScaleReport } from '../types/api';
import GlassSection from './GlassSection';

export default function CapacityScalePanel() {
  const [report, setReport] = useState<CapacityScaleReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [executing, setExecuting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<CapacityScaleReport>('/intelligence/capacity/scale-suggestions');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function execute(dryRun: boolean) {
    setExecuting(true);
    const res = await apiPost<CapacityScaleExecuteReport>('/intelligence/capacity/scale/execute', {
      dry_run: dryRun,
    });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Scale dry-run: ${res.data?.executed.length ?? 0} suggestions`
              : `Scale applied: ${res.data?.executed.length ?? 0} actions`,
            type: 'success',
          },
        }),
      );
    }
  }

  return (
        <GlassSection
      accent="blue"
      testId="capacity-scale-panel"
      title="Capacity Auto-Scale"
      subtitle="Predictive HPA/VPA suggestions from failure signals"
      icon={<Gauge className="h-5 w-5 text-amber-400" />}
      actions={<div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void execute(true)}
            disabled={executing}
            className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-200 hover:border-amber-400/50 disabled:opacity-60"
            data-testid="capacity-scale-dry-run"
          >
            Dry-run scale
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>}
    >{!report?.suggestions.length ? (
        <p className="text-sm text-slate-500">No scaling suggestions — capacity headroom looks adequate.</p>
      ) : (
        <ul className="space-y-2">
          {report.suggestions.slice(0, 6).map((s) => (
            <li
              key={`${s.workload}-${s.kind}-${s.resource}`}
              className="rounded-lg border glass-divider px-3 py-2 text-sm"
            >
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium text-white">{s.workload}</span>
                <Badge text={s.kind.toUpperCase()} variant="muted" />
                {s.auto_safe ? <Badge text="auto-safe" variant="green" /> : null}
              </div>
              <p className="mt-1 text-slate-400">
                {s.resource}: {s.current} → {s.suggested}
              </p>
              <p className="mt-1 text-xs text-slate-500">{s.reason}</p>
            </li>
          ))}
        </ul>
      )}
    </GlassSection>
  );
}
