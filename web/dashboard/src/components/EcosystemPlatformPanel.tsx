// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Boxes, Loader2, RefreshCw, Rocket, Store } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import GlassSection from './GlassSection';

interface SaasTenants {
  billing_period: string;
  tenants: Array<{ slug: string; plan: string; workload_count: number }>;
  total_workloads: number;
}

interface PluginMarketplace {
  entries: Array<{ name: string; version: string; installed: boolean }>;
}

interface PublicApi {
  version: string;
  routes: Array<{ method: string; path: string }>;
}

interface AutonomousSre {
  closed_loop_ready: boolean;
  autonomy_enabled: boolean;
  human_gate_required: boolean;
  agents_active: number;
}

interface CommunityIntents {
  built_in_count: number;
  community: Array<{ id: string; title: string; stars: number }>;
}

export default function EcosystemPlatformPanel() {
  const [tab, setTab] = useState<'saas' | 'plugins' | 'api' | 'sre' | 'community'>('saas');
  const [loading, setLoading] = useState(true);
  const [saas, setSaas] = useState<SaasTenants | null>(null);
  const [plugins, setPlugins] = useState<PluginMarketplace | null>(null);
  const [publicApi, setPublicApi] = useState<PublicApi | null>(null);
  const [sre, setSre] = useState<AutonomousSre | null>(null);
  const [community, setCommunity] = useState<CommunityIntents | null>(null);
  const [helmPreview, setHelmPreview] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [st, pm, api, asre, ci] = await Promise.all([
      apiFetch<SaasTenants>('/intelligence/platform/saas-tenants'),
      apiFetch<PluginMarketplace>('/intelligence/platform/plugin-marketplace'),
      apiFetch<PublicApi>('/intelligence/platform/public-api'),
      apiFetch<AutonomousSre>('/intelligence/platform/autonomous-sre'),
      apiFetch<CommunityIntents>('/intelligence/platform/community-intents'),
    ]);
    setSaas(st);
    setPlugins(pm);
    setPublicApi(api);
    setSre(asre);
    setCommunity(ci);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function generateHelm() {
    const res = await apiPost<{ chart_yaml: string; values_yaml: string; workload: string }>(
      '/intelligence/platform/helm-v2',
      { goals: ['cost-optimized', 'ha'], workload_name: 'platform-demo' },
    );
    if (res.success && res.data) {
      setHelmPreview(`${res.data.workload}\n---\n${res.data.chart_yaml.slice(0, 200)}…`);
    }
  }

  async function runAutonomousSre() {
    const res = await apiPost<{ healer: string[]; finops: string[]; gitops: string[] }>(
      '/intelligence/platform/autonomous-sre/execute',
      { dry_run: true },
    );
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: `Autonomous SRE dry-run: ${(res.data?.healer.length ?? 0) + (res.data?.finops.length ?? 0)} actions`,
            type: 'success',
          },
        }),
      );
    }
  }

  const tabs = [
    { id: 'saas' as const, label: 'SaaS tenants' },
    { id: 'plugins' as const, label: 'Plugins' },
    { id: 'api' as const, label: 'Public API' },
    { id: 'sre' as const, label: 'Autonomous SRE' },
    { id: 'community' as const, label: 'Intents' },
  ];

  return (
    <GlassSection
      accent="blue"
      testId="ecosystem-platform-panel"
      title="Platform & Ecosystem"
      subtitle="Multi-tenant SaaS, plugin marketplace, Helm v2, public v1 API, autonomous SRE"
      icon={<Boxes className="h-5 w-5 text-blue-400" />}
      actions={
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void generateHelm()}
            className="inline-flex items-center gap-1 rounded-xl border border-blue-500/30 bg-blue-500/10 px-3 py-2 text-xs text-blue-200"
            data-testid="ecosystem-helm-v2"
          >
            <Rocket className="h-3.5 w-3.5" />
            Helm v2
          </button>
          <button
            type="button"
            onClick={() => void runAutonomousSre()}
            className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-200"
            data-testid="ecosystem-sre-execute"
          >
            SRE dry-run
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>
      }
    >
      {helmPreview ? (
        <pre className="mb-4 max-h-24 overflow-auto rounded-lg border border-slate-800 bg-slate-950/50 p-2 text-xs text-slate-400">
          {helmPreview}
        </pre>
      ) : null}

      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-blue-500/40 bg-blue-500/10 text-blue-200' : 'border-slate-700 text-slate-400'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'saas' ? (
        <div data-testid="ecosystem-saas-panel" className="text-sm text-slate-300">
          <p>
            Period {saas?.billing_period ?? '—'} · {saas?.total_workloads ?? 0} workload(s) ·{' '}
            {saas?.tenants.length ?? 0} tenant(s)
          </p>
        </div>
      ) : null}

      {tab === 'plugins' ? (
        <div data-testid="ecosystem-plugins-panel">
          <div className="mb-2 flex items-center gap-2 text-sm text-slate-300">
            <Store className="h-4 w-4" />
            {plugins?.entries.length ?? 0} plugin(s)
          </div>
          <ul className="space-y-1 text-xs text-slate-400">
            {(plugins?.entries ?? []).slice(0, 6).map((p) => (
              <li key={p.name}>
                {p.name} v{p.version} {p.installed ? '· installed' : '· catalog'}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'api' ? (
        <div data-testid="ecosystem-public-api-panel" className="text-sm text-slate-300">
          <p className="mb-2">Public AI OS API {publicApi?.version ?? '—'}</p>
          <ul className="space-y-1 text-xs font-mono text-slate-400">
            {(publicApi?.routes ?? []).slice(0, 5).map((r) => (
              <li key={r.path}>
                {r.method} {r.path}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'sre' ? (
        <div data-testid="ecosystem-sre-panel" className="text-sm text-slate-300">
          <p>
            Closed loop: {sre?.closed_loop_ready ? 'ready' : 'gated'} · agents {sre?.agents_active ?? 0}
            {sre?.human_gate_required ? ' · human gate active' : ''}
          </p>
        </div>
      ) : null}

      {tab === 'community' ? (
        <div data-testid="ecosystem-community-panel" className="text-sm text-slate-300">
          <p className="mb-2">
            {community?.built_in_count ?? 0} built-in + {community?.community.length ?? 0} community template(s)
          </p>
          <ul className="text-xs text-slate-400">
            {(community?.community ?? []).map((c) => (
              <li key={c.id}>
                {c.title} ★{c.stars}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </GlassSection>
  );
}
