// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Layers, Loader2, RefreshCw, Sparkles } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import Badge from './Badge';
import type {
  IntentBundleReport,
  IntentTemplateLibrary,
  IntentVersionHistory,
  IntentViolationsReport,
} from '../types/api';

export default function IntentPlatformPanel() {
  const [tab, setTab] = useState<'violations' | 'templates' | 'bundles' | 'versions'>('violations');
  const [loading, setLoading] = useState(true);
  const [violations, setViolations] = useState<IntentViolationsReport | null>(null);
  const [templates, setTemplates] = useState<IntentTemplateLibrary | null>(null);
  const [bundle, setBundle] = useState<IntentBundleReport | null>(null);
  const [versionWorkload, setVersionWorkload] = useState('my-app');
  const [versions, setVersions] = useState<IntentVersionHistory | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [v, t] = await Promise.all([
      apiFetch<IntentViolationsReport>('/intelligence/intent/violations'),
      apiFetch<IntentTemplateLibrary>('/intelligence/intent/templates'),
    ]);
    setViolations(v);
    setTemplates(t);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function loadBundle() {
    const res = await apiPost<IntentBundleReport>('/intelligence/intent/bundles', {
      bundle_name: 'platform',
      goal: 'balanced',
    });
    if (res.success) setBundle(res.data ?? null);
  }

  async function loadVersions() {
    const data = await apiFetch<IntentVersionHistory>(
      `/intelligence/intent/versions/${encodeURIComponent(versionWorkload.trim() || 'my-app')}`,
    );
    setVersions(data);
  }

  const tabs = [
    { id: 'violations' as const, label: 'Violations' },
    { id: 'templates' as const, label: 'Templates' },
    { id: 'bundles' as const, label: 'Bundles' },
    { id: 'versions' as const, label: 'Versions' },
  ];

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="intent-platform-panel">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Layers className="h-5 w-5 text-violet-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Intent Platform</h2>
            <p className="text-sm text-slate-400">Violations, templates, bundles, and version history</p>
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

      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-violet-500/40 bg-violet-500/10 text-violet-200' : 'border-slate-700 text-slate-400'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'violations' ? (
        <div data-testid="intent-violations-panel">
          <p className="mb-3 text-xs text-slate-500">
            Reconciliation: {violations?.reconciliation_status ?? '—'}
          </p>
          {!violations?.violations.length ? (
            <p className="text-sm text-slate-500">No intent violations — fleet intent is in sync.</p>
          ) : (
            <ul className="space-y-2">
              {violations.violations.map((v) => (
                <li key={`${v.workload}-${v.violation_type}`} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-medium text-white">{v.workload}</span>
                    <Badge text={v.violation_type} variant="muted" />
                    <Badge text={v.severity} variant={v.severity === 'critical' ? 'red' : 'yellow'} />
                  </div>
                  <p className="mt-1 text-xs text-slate-400">{v.detail}</p>
                  <p className="text-xs text-slate-500">
                    {v.current_value} → target {v.intent_target}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}

      {tab === 'templates' ? (
        <ul className="space-y-3" data-testid="intent-templates-panel">
          {(templates?.templates ?? []).map((tpl) => (
            <li key={tpl.id} className="rounded-xl border border-slate-800 p-4">
              <div className="flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-violet-400" />
                <span className="font-medium text-white">{tpl.title}</span>
                <Badge text={tpl.goal} variant="muted" />
              </div>
              <p className="mt-1 text-sm text-slate-400">{tpl.description}</p>
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'bundles' ? (
        <div data-testid="intent-bundles-panel">
          <button
            type="button"
            onClick={() => void loadBundle()}
            className="mb-4 rounded-xl border border-violet-500/30 bg-violet-500/10 px-3 py-2 text-xs text-violet-200"
          >
            Generate app + db + cache bundle
          </button>
          {bundle ? (
            <ul className="space-y-2">
              {bundle.workloads.map((w) => (
                <li key={w.workload_name} className="rounded-lg border border-slate-800 px-3 py-2 text-sm">
                  <Badge text={w.role} variant="muted" /> {w.workload_name}
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}

      {tab === 'versions' ? (
        <div data-testid="intent-versions-panel">
          <div className="mb-3 flex gap-2">
            <input
              value={versionWorkload}
              onChange={(e) => setVersionWorkload(e.target.value)}
              className="rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-200"
              placeholder="Workload name"
            />
            <button
              type="button"
              onClick={() => void loadVersions()}
              className="rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300"
            >
              Load history
            </button>
          </div>
          {!versions?.versions.length ? (
            <p className="text-sm text-slate-500">No intent versions recorded yet.</p>
          ) : (
            <ul className="space-y-2">
              {versions.versions.map((v) => (
                <li key={v.version_id} className="rounded-lg border border-slate-800 px-3 py-2 text-sm text-slate-300">
                  {v.version_id} · {v.goal} · {v.recorded_at}
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </section>
  );
}
