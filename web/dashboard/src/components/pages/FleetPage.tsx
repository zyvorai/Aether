// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { ExternalLink, Globe, Network, Server, Shield } from 'lucide-react';
import { useNavigate } from 'react-router';
import { apiFetchSettled } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';
import StatCard from '../StatCard';
import type { ClusterSummary } from '../../types/api';

interface Integrations {
  hubble_ui_url?: string | null;
  packetwolf_url?: string | null;
  grafana_url?: string | null;
  prometheus_url?: string | null;
}

interface ServerPayload {
  integrations?: Integrations;
}

export default function FleetPage() {
  const navigate = useNavigate();
  const [summary, setSummary] = useState<ClusterSummary | null>(null);
  const [integrations, setIntegrations] = useState<Integrations>({});
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [clusterRes, serverRes] = await Promise.all([
      apiFetchSettled<ClusterSummary>('/cluster/summary'),
      apiFetchSettled<ServerPayload>('/server'),
    ]);
    if (!clusterRes.ok && !serverRes.ok) {
      setLoadFailed(true);
      setSummary(null);
    } else {
      setSummary(clusterRes.ok ? clusterRes.data : null);
      setIntegrations(serverRes.ok ? (serverRes.data.integrations ?? {}) : {});
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && !summary && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Fleet overview unavailable" onRetry={() => void load()} />;
  }

  const clusters = summary?.clusters ?? [];

  return (
    <div>
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <StatCard title="Clusters" value={summary?.cluster_count ?? 0} color="blue" icon={<Globe size={18} />} />
        <StatCard title="Healthy" value={summary?.healthy_clusters ?? 0} color="green" icon={<Shield size={18} />} />
        <button type="button" onClick={() => navigate(viewToPath('workloads'))} className="text-left">
          <StatCard title="Workloads" value={summary?.workload_count ?? 0} color="purple" icon={<Server size={18} />} />
        </button>
        <StatCard
          title="Backend"
          value={summary?.connected ? 'connected' : 'offline'}
          color={summary?.connected ? 'green' : 'red'}
          icon={<Network size={18} />}
        />
      </div>

      {summary?.error && (
        <div className="mb-4 rounded-xl border border-amber-800/50 bg-amber-950/30 px-4 py-3 text-sm text-amber-200">
          Kubeconfig inventory: {summary.error}
        </div>
      )}

      <div className="dash-card mb-6">
        <h2 className="text-lg font-semibold text-slate-100 mb-4">Registered clusters</h2>
        {clusters.length === 0 ? (
          <p className="text-sm text-slate-500">
            No clusters in kubeconfig inventory. Configure kubeconfig on the API server or open{' '}
            <button type="button" onClick={() => navigate(viewToPath('clusters'))} className="text-aether hover:underline">
              Cluster Browser
            </button>
            .
          </p>
        ) : (
          <div className="space-y-3">
            {clusters.map((c) => (
              <button
                key={c.name}
                type="button"
                data-testid={`fleet-cluster-${c.name}`}
                onClick={() => navigate(pathWithQuery(viewToPath('clusters'), { cluster: c.name }))}
                className="w-full text-left rounded-xl border border-slate-800 px-4 py-3 hover:border-aether/40 transition-colors"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-medium text-slate-100">{c.name}</span>
                  <Badge text={c.reachable ? 'reachable' : 'unreachable'} variant={c.reachable ? 'green' : 'red'} />
                </div>
                <p className="text-xs text-slate-500 mt-1 font-mono truncate">{c.server ?? '—'}</p>
                {c.version && <p className="text-xs text-slate-400 mt-1">Kubernetes {c.version}</p>}
              </button>
            ))}
          </div>
        )}
        <p className="mt-4 text-xs text-slate-500">
          Backend: {summary?.backend ?? '—'}
          {summary?.summary_note ? ` · ${summary.summary_note}` : ''}
        </p>
      </div>

      <div className="dash-card">
        <h2 className="text-lg font-semibold text-slate-100 mb-4">Network observability</h2>
        <p className="text-sm text-slate-500 mb-4">
          Deep Hubble flow queries and PacketWolf east-west verification are integrated via env URLs on the control plane.
        </p>
        <div className="flex flex-wrap gap-3">
          {integrations.grafana_url ? (
            <a
              href={integrations.grafana_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm text-aether hover:bg-aether/20"
            >
              Grafana <ExternalLink size={14} />
            </a>
          ) : null}
          {integrations.prometheus_url ? (
            <a
              href={integrations.prometheus_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-200 hover:bg-slate-800"
            >
              Prometheus <ExternalLink size={14} />
            </a>
          ) : null}
          {integrations.hubble_ui_url ? (
            <a
              href={integrations.hubble_ui_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-purple-700/50 bg-purple-950/30 px-4 py-2 text-sm text-purple-200 hover:bg-purple-950/50"
            >
              Hubble UI <ExternalLink size={14} />
            </a>
          ) : (
            <span className="text-sm text-slate-500 self-center">
              Set <code className="text-slate-400">AETHER_HUBBLE_UI_URL</code> for Hubble
            </span>
          )}
          {integrations.packetwolf_url ? (
            <a
              href={integrations.packetwolf_url}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-cyan-700/50 bg-cyan-950/30 px-4 py-2 text-sm text-cyan-200 hover:bg-cyan-950/50"
            >
              PacketWolf <ExternalLink size={14} />
            </a>
          ) : (
            <span className="text-sm text-slate-500 self-center">
              Set <code className="text-slate-400">AETHER_PACKETWOLF_URL</code> for PacketWolf
            </span>
          )}
          <button
            type="button"
            onClick={() => navigate(viewToPath('platform'))}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800"
          >
            Platform & HA settings
          </button>
        </div>
      </div>
    </div>
  );
}
