// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { FlaskConical, Loader2, RefreshCw, Sparkles } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import GlassSection from './GlassSection';

interface LabsOverview {
  graduated_count: number;
  features: Array<{ phase: number; name: string; status: string; endpoint: string }>;
}

interface LabsTerraform {
  status: string;
  workload: string;
  hcl: string;
  modules: string[];
}

interface LabsCommunity {
  status: string;
  built_in_count: number;
  community: Array<{ id: string; title: string; stars: number }>;
  imported_count: number;
}

interface LabsCarbon {
  status: string;
  fleet_carbon_kg_monthly: number;
  workloads: Array<{ workload: string; carbon_kg_monthly: number }>;
}

export default function LabsGraduationPanel() {
  const [tab, setTab] = useState<'overview' | 'terraform' | 'community' | 'carbon' | 'voice'>('overview');
  const [loading, setLoading] = useState(true);
  const [overview, setOverview] = useState<LabsOverview | null>(null);
  const [terraform, setTerraform] = useState<LabsTerraform | null>(null);
  const [community, setCommunity] = useState<LabsCommunity | null>(null);
  const [carbon, setCarbon] = useState<LabsCarbon | null>(null);
  const [voiceSupported, setVoiceSupported] = useState(false);
  const [graphNodes, setGraphNodes] = useState(0);

  const load = useCallback(async () => {
    setLoading(true);
    const [ov, ci, cb, vc, ge] = await Promise.all([
      apiFetch<LabsOverview>('/intelligence/labs/overview'),
      apiFetch<LabsCommunity>('/intelligence/labs/community-intents'),
      apiFetch<LabsCarbon>('/intelligence/labs/carbon'),
      apiFetch<{ supported: boolean }>('/intelligence/labs/voice-copilot'),
      apiFetch<{ node_count: number; status: string }>('/intelligence/labs/graph-export?format=neo4j'),
    ]);
    setOverview(ov);
    setCommunity(ci);
    setCarbon(cb);
    setVoiceSupported(vc?.supported ?? false);
    setGraphNodes(ge?.node_count ?? 0);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function exportTerraform() {
    const res = await apiPost<LabsTerraform>('/intelligence/labs/terraform-export', {
      goals: ['cost-optimized'],
      workload_name: 'labs-demo',
    });
    if (res.success && res.data) setTerraform(res.data);
  }

  async function importCommunityTemplate() {
    await apiPost('/intelligence/labs/community-intents/import', {
      id: 'custom-labs-intent',
      title: 'Custom labs intent',
      author: 'dashboard',
      goal: 'balanced',
    });
    const ci = await apiFetch<LabsCommunity>('/intelligence/labs/community-intents');
    setCommunity(ci);
  }

  const tabs = [
    { id: 'overview' as const, label: 'Overview' },
    { id: 'terraform' as const, label: 'Terraform' },
    { id: 'community' as const, label: 'Community' },
    { id: 'carbon' as const, label: 'Carbon' },
    { id: 'voice' as const, label: 'Voice' },
  ];

  return (
    <GlassSection
      title="Lab Graduation"
      subtitle="Era K — former Lab features now shipped with production APIs"
      icon={<FlaskConical className="h-5 w-5 text-aether-ai" />}
      testId="labs-graduation-panel"
      actions={
        <button type="button" onClick={() => void load()} className="btn-secondary text-xs">
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh
        </button>
      }
    >
      <div className="mb-4 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={tab === t.id ? 'glass-tab-active tab-chip-active' : 'glass-tab tab-chip'}
          >
            {t.label}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-sm text-slate-400">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading lab graduation…
        </div>
      ) : null}

      {!loading && tab === 'overview' && overview ? (
        <div data-testid="labs-overview-panel">
          <p className="text-sm text-slate-400 mb-3">
            <Sparkles className="inline h-4 w-4 text-aether-ai mr-1" />
            {overview.graduated_count} features graduated · graph export {graphNodes} nodes
          </p>
          <ul className="space-y-2 text-sm">
            {overview.features.map((f) => (
              <li
                key={f.phase}
                className="flex items-center justify-between rounded-xl border glass-divider glass-panel-card px-3 py-2"
              >
                <span className="text-slate-200">
                  Phase {f.phase}: {f.name}
                </span>
                <span className="text-xs text-emerald-400 uppercase">{f.status}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {!loading && tab === 'terraform' ? (
        <div data-testid="labs-terraform-panel" className="space-y-3">
          <button type="button" onClick={() => void exportTerraform()} className="btn-primary text-xs">
            Generate Terraform v2
          </button>
          {terraform ? (
            <pre className="glass-panel-card overflow-x-auto p-3 text-xs text-slate-300 font-mono max-h-48">
              {terraform.hcl.slice(0, 400)}…
            </pre>
          ) : (
            <p className="text-sm text-slate-500">Click to generate OpenTofu-ready HCL from intent pipeline.</p>
          )}
        </div>
      ) : null}

      {!loading && tab === 'community' && community ? (
        <div data-testid="labs-community-panel" className="space-y-3">
          <p className="text-sm text-slate-400">
            {community.built_in_count} built-in · {community.community.length} community ·{' '}
            {community.imported_count} imported
          </p>
          <button type="button" onClick={() => void importCommunityTemplate()} className="btn-secondary text-xs">
            Import sample template
          </button>
          <ul className="space-y-1 text-sm text-slate-300">
            {community.community.slice(0, 5).map((c) => (
              <li key={c.id}>
                {c.title} <span className="text-slate-500">★ {c.stars}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {!loading && tab === 'carbon' && carbon ? (
        <div data-testid="labs-carbon-panel">
          <p className="text-sm text-slate-300">
            Fleet carbon: <strong>{carbon.fleet_carbon_kg_monthly.toFixed(2)}</strong> kg CO₂e / month
          </p>
          {carbon.workloads.length > 0 ? (
            <ul className="mt-2 text-xs text-slate-500 space-y-1">
              {carbon.workloads.slice(0, 4).map((w) => (
                <li key={w.workload}>
                  {w.workload}: {w.carbon_kg_monthly.toFixed(2)} kg/mo
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-xs text-slate-500 mt-2">Deploy workloads to see per-line carbon estimates.</p>
          )}
        </div>
      ) : null}

      {!loading && tab === 'voice' ? (
        <div data-testid="labs-voice-panel">
          <p className="text-sm text-slate-300">
            Web Speech API: {voiceSupported ? 'supported in this browser' : 'check browser compatibility'}
          </p>
          <p className="text-xs text-slate-500 mt-2">
            Use microphone input in Copilot or pipe transcripts to POST /api/copilot/chat.
          </p>
        </div>
      ) : null}
    </GlassSection>
  );
}
