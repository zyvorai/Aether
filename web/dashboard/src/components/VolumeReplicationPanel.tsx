// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { HardDrive, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import Badge from './Badge';
import type { VolumeReplicationStatusReport } from '../types/api';

export default function VolumeReplicationPanel() {
  const [report, setReport] = useState<VolumeReplicationStatusReport | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<VolumeReplicationStatusReport>('/intelligence/migration/volume-status');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="volume-replication-panel">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <HardDrive className="h-5 w-5 text-violet-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Volume Replication</h2>
            <p className="text-sm text-slate-400">CSI executor readiness across persistent workloads</p>
          </div>
        </div>
        <button type="button" onClick={() => void load()} className="rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300">
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
        </button>
      </div>
      {!report?.entries.length ? (
        <p className="text-sm text-slate-500">No persistent workloads in fleet.</p>
      ) : (
        <ul className="space-y-2">
          {report.entries.map((e) => (
            <li key={e.workload} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium text-white">{e.workload}</span>
                {e.plan_ready ? <Badge text="plan ready" variant="green" /> : <Badge text="N/A" variant="muted" />}
              </div>
              <p className="mt-1 text-xs text-slate-400">{e.summary}</p>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
