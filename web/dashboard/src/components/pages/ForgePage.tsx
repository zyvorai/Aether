// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { ForgeStats, ForgeNode } from '../../types/api';

function num(v: unknown): string {
  return typeof v === 'number' ? String(v) : v == null ? '—' : String(v);
}

export default function ForgePage() {
  const [stats, setStats] = useState<ForgeStats | null>(null);
  const [nodes, setNodes] = useState<ForgeNode[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [statsRes, nodesRes] = await Promise.all([
      apiFetchSettled<ForgeStats>('/forge/stats'),
      apiFetchSettled<ForgeNode[]>('/forge/nodes'),
    ]);
    if (!statsRes.ok && !nodesRes.ok) {
      setLoadFailed(true);
    } else {
      setStats(statsRes.ok ? statsRes.data : null);
      setNodes(nodesRes.ok ? nodesRes.data : []);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && !stats && nodes.length === 0 && !loadFailed) {
    return <PageLoading rows={4} />;
  }
  if (loadFailed) {
    return <PageLoadError title="Forge unavailable" onRetry={() => void load()} />;
  }

  const notConfigured = stats !== null && stats.configured === false;

  return (
    <div>
      <section className="overview-section-shell mb-6 p-6 sm:p-8">
        <PageToolbar onRefresh={() => void load()} refreshing={loading} />

        {notConfigured ? (
          <EmptyState
            icon={<Inbox size={48} />}
            title="Forge not configured"
            description="Set AETHER_FORGE_URL (and AETHER_FORGE_TOKEN) on the gateway to view GPU capacity, nodes, and placement from the Forge control plane."
          />
        ) : (
          <>
            <div className="mb-6 grid grid-cols-2 gap-3 md:grid-cols-4" data-testid="forge-stats">
              {[
                ['Total GPUs', num(stats?.totalGPUs)],
                ['Available', num(stats?.availableGPUs)],
                ['Allocated', num(stats?.allocatedGPUs)],
                ['Utilization %', num(stats?.utilizationPercent)],
              ].map(([label, value]) => (
                <div key={label} className="glass-panel-card px-4 py-3">
                  <div className="text-xs uppercase tracking-wider text-slate-500">{label}</div>
                  <div className="mt-2 text-lg font-semibold text-slate-100">{value}</div>
                </div>
              ))}
            </div>

            {nodes.length === 0 ? (
              <EmptyState
                icon={<Inbox size={48} />}
                title="No GPU nodes"
                description="Forge reports no GPU-capable nodes on this cluster."
              />
            ) : (
              <div className="glass-panel-card overflow-hidden" data-testid="forge-nodes">
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="glass-divider-b">
                        <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Node</th>
                        <th className="text-right text-xs uppercase tracking-wider text-slate-500 py-3 px-4">GPUs</th>
                      </tr>
                    </thead>
                    <tbody>
                      {nodes.map((n, i) => (
                        <tr key={(n.metadata?.name ?? '') + i} className="glass-table-row glass-inset-hover transition-colors">
                          <td className="py-3 px-4 font-medium text-slate-200">{n.metadata?.name ?? '—'}</td>
                          <td className="py-3 px-4 text-right text-sm text-slate-300">{num(n.spec?.gpuCount)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </>
        )}
      </section>
    </div>
  );
}
