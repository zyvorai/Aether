// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useState } from 'react';
import { GitBranch, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import Badge from './Badge';
import type { IntentGitOpsDiffReport } from '../types/api';
import GlassSection from './GlassSection';

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
    <GlassSection
      accent="purple"
      testId="intent-gitops-diff-panel"
      title="Intent GitOps diff"
      subtitle="Intent block drift separate from spec drift"
      icon={<GitBranch className="h-5 w-5 text-warning" />}
      actions={
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-muted hover:border-primary/40"
        >
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          Refresh
        </button>
      }
    >
      {!report?.entries.length ? (
        <p className="text-sm text-subtle">No workloads with intent blocks tracked.</p>
      ) : (
        <ul className="space-y-3">
          {report.entries.slice(0, 6).map((entry) => (
            <li key={entry.workload} className="rounded-xl border glass-divider px-4 py-3">
              <div className="mb-2 flex flex-wrap items-center gap-2">
                <span className="font-medium text-foreground">{entry.workload}</span>
                {entry.has_drift ? <Badge text="drift" variant="yellow" /> : <Badge text="synced" variant="green" />}
              </div>
              <p className="text-xs text-muted">{entry.summary}</p>
              <pre className="mt-2 max-h-24 overflow-auto rounded-lg glass-code-block-body p-2 text-[11px] text-muted">
                {entry.live_intent_yaml}
              </pre>
            </li>
          ))}
        </ul>
      )}
    </GlassSection>
  );
}
