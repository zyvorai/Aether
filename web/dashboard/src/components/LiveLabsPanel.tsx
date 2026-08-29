// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Loader2, RefreshCw, Server } from 'lucide-react';
import { apiFetch } from '../utils/api';
import GlassSection from './GlassSection';

interface LiveLabsOverview {
  feature_count: number;
  labs_live_enabled: boolean;
  kubeconfig_available: boolean;
  era: string;
}

interface ReferenceRunner {
  labs_live_enabled: boolean;
  kubeconfig_available: boolean;
  make_target: string;
  script: string;
  hint: string;
}

interface LiveSmoke {
  steps: Array<{ id: string; script: string }>;
}

interface CiPipeline {
  jobs: Array<{ id: string; description: string; command: string }>;
}

export default function LiveLabsPanel() {
  const [tab, setTab] = useState<'runner' | 'fixtures' | 'pipeline'>('runner');
  const [loading, setLoading] = useState(true);
  const [overview, setOverview] = useState<LiveLabsOverview | null>(null);
  const [runner, setRunner] = useState<ReferenceRunner | null>(null);
  const [smoke, setSmoke] = useState<LiveSmoke | null>(null);
  const [ci, setCi] = useState<CiPipeline | null>(null);
  const [k8sSpecs, setK8sSpecs] = useState(0);
  const [confidentialSpecs, setConfidentialSpecs] = useState(0);

  const load = useCallback(async () => {
    setLoading(true);
    const [ov, rr, ls, cp, kl, cl] = await Promise.all([
      apiFetch<LiveLabsOverview>('/intelligence/livelabs/overview'),
      apiFetch<ReferenceRunner>('/intelligence/livelabs/reference-runner'),
      apiFetch<LiveSmoke>('/intelligence/livelabs/live-smoke'),
      apiFetch<CiPipeline>('/intelligence/livelabs/ci-pipeline'),
      apiFetch<{ specs: unknown[] }>('/intelligence/livelabs/kubernetes-lab'),
      apiFetch<{ specs: unknown[] }>('/intelligence/livelabs/confidential-lab'),
    ]);
    setOverview(ov);
    setRunner(rr);
    setSmoke(ls);
    setCi(cp);
    setK8sSpecs(kl?.specs?.length ?? 0);
    setConfidentialSpecs(cl?.specs?.length ?? 0);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <GlassSection
      title="Live Labs & Reference Cluster"
      subtitle="Era N — kubeconfig gates, kind fixtures, CI smoke, and post-deploy verify"
      icon={<Server className="h-5 w-5 text-sky-400" />}
      testId="live-labs-panel"
      actions={
        <button type="button" onClick={() => void load()} className="btn-secondary text-xs">
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh
        </button>
      }
    >
      <div className="mb-4 flex flex-wrap gap-2">
        {(['runner', 'fixtures', 'pipeline'] as const).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={tab === t ? 'glass-tab-active tab-chip-active' : 'glass-tab tab-chip'}
          >
            {t === 'runner' ? 'Runner' : t === 'fixtures' ? 'Fixtures' : 'Pipeline'}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-sm text-ink-2">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading live labs…
        </div>
      ) : null}

      {!loading && tab === 'runner' && runner && overview ? (
        <div data-testid="live-labs-runner-panel" className="text-sm text-ink-2 space-y-2">
          <p>
            Era {overview.era} · {overview.feature_count} features · live{' '}
            {overview.labs_live_enabled ? 'on' : 'off'} · kubeconfig{' '}
            {overview.kubeconfig_available ? 'ready' : 'missing'}
          </p>
          <p>
            Make target: <code className="text-sky-300">{runner.make_target}</code>
          </p>
          <p>
            Script: <code className="text-sky-300">{runner.script}</code>
          </p>
          <p className="text-ink-2">{runner.hint}</p>
          {smoke ? (
            <ul className="mt-2 space-y-1 text-xs text-ink-3">
              {smoke.steps.map((s) => (
                <li key={s.id}>
                  {s.id}: {s.script}
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}

      {!loading && tab === 'fixtures' ? (
        <div data-testid="live-labs-fixtures-panel" className="text-sm text-ink-2 space-y-2">
          <p>
            Kubernetes lab specs: {k8sSpecs} · Confidential specs: {confidentialSpecs}
          </p>
          <p className="text-ink-2">
            Kind fixture: <code className="text-sky-300">scripts/kind-playwright-fixture.sh</code>
          </p>
          <p className="text-ink-2">
            Post-deploy: <code className="text-sky-300">scripts/post-deploy-verify.sh</code>
          </p>
        </div>
      ) : null}

      {!loading && tab === 'pipeline' && ci ? (
        <div data-testid="live-labs-pipeline-panel">
          <ul className="space-y-2 text-sm">
            {ci.jobs.map((j) => (
              <li
                key={j.id}
                className="flex flex-col rounded-xl border glass-divider glass px-3 py-2"
              >
                <span className="text-ink">{j.description}</span>
                <span className="text-xs text-ink-3 font-mono mt-1">{j.command}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </GlassSection>
  );
}
