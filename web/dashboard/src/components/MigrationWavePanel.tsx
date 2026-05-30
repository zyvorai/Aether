// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Layers, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import Badge from './Badge';
import type { MigrationWaveReport } from '../types/api';

export default function MigrationWavePanel() {
  const [report, setReport] = useState<MigrationWaveReport | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<MigrationWaveReport>('/intelligence/migration/wave-plan');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="migration-wave-panel">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Layers className="h-5 w-5 text-emerald-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Migration Wave Planner</h2>
            <p className="text-sm text-slate-400">Multi-cluster coordinated migration waves</p>
          </div>
        </div>
        <button type="button" onClick={() => void load()} className="rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300">
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
        </button>
      </div>
      {report ? (
        <>
          <div className="mb-4 flex flex-wrap gap-2">
            <Badge text={`target: ${report.target_cluster}`} variant="muted" />
            <Badge text={`${report.wave_size} workloads`} variant="green" />
            <Badge text={report.plan.strategy} variant="muted" />
          </div>
          <ul className="space-y-2">
            {report.plan.items.slice(0, 8).map((item) => (
              <li key={item.workload} className="rounded-lg border border-slate-800 px-3 py-2 text-sm text-slate-300">
                {item.workload}: {item.source_runtime} → {item.target_runtime} @ {item.target_cluster}
                {item.volume_replication ? <Badge text="volume sync" variant="yellow" /> : null}
              </li>
            ))}
          </ul>
        </>
      ) : (
        <p className="text-sm text-slate-500">No migration wave planned.</p>
      )}
    </section>
  );
}
