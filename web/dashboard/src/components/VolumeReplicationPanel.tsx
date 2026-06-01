// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { HardDrive, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import Badge from './Badge';
import type { VolumeReplicationStatusReport } from '../types/api';
import GlassSection from './GlassSection';

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
    <GlassSection
      accent="blue"
      testId="volume-replication-panel"
      title="Volume Replication"
      subtitle="CSI executor readiness across persistent workloads"
      icon={<HardDrive className="h-5 w-5 text-violet-400" />}
      actions={
        <button type="button" onClick={() => void load()} className="rounded-xl border glass-divider px-3 py-2 text-xs text-slate-300">
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
        </button>
      }
    >
      {!report?.entries.length ? (
        <p className="text-sm text-slate-500">No persistent workloads in fleet.</p>
      ) : (
        <ul className="space-y-2">
          {report.entries.map((e) => (
            <li key={e.workload} className="rounded-lg border glass-divider px-3 py-2 text-sm">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium text-white">{e.workload}</span>
                {e.plan_ready ? <Badge text="plan ready" variant="green" /> : <Badge text="N/A" variant="muted" />}
              </div>
              <p className="mt-1 text-xs text-slate-400">{e.summary}</p>
            </li>
          ))}
        </ul>
      )}
    </GlassSection>
  );
}
