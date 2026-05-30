// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { GitBranch, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import Badge from './Badge';
import type { IntentGitOpsDiffReport } from '../types/api';

export default function IntentGitOpsDiffPanel() {
  const [report, setReport] = useState<IntentGitOpsDiffReport | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<IntentGitOpsDiffReport>('/intelligence/intent/gitops-diff');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="intent-gitops-diff-panel">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <GitBranch className="h-5 w-5 text-amber-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Intent GitOps diff</h2>
            <p className="text-sm text-slate-400">Intent block drift separate from spec drift</p>
          </div>
        </div>
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
        >
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          Refresh
        </button>
      </div>

      {!report?.entries.length ? (
        <p className="text-sm text-slate-500">No workloads with intent blocks tracked.</p>
      ) : (
        <ul className="space-y-3">
          {report.entries.slice(0, 6).map((entry) => (
            <li key={entry.workload} className="rounded-xl border border-slate-800 px-4 py-3">
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <span className="font-medium text-white">{entry.workload}</span>
                {entry.has_drift ? <Badge text="drift" variant="yellow" /> : <Badge text="synced" variant="green" />}
              </div>
              <p className="text-xs text-slate-400">{entry.summary}</p>
              <pre className="mt-2 max-h-24 overflow-auto rounded-lg bg-black/40 p-2 text-[11px] text-slate-400">
                {entry.live_intent_yaml}
              </pre>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
