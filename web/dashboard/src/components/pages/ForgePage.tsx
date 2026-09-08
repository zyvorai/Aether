// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { useState, useEffect, useCallback } from 'react';
import { Cpu, Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import StatRibbon from '../StatRibbon';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { ForgeStats, ForgeNode } from '../../types/api';

function num(v: unknown): string {
  return typeof v === 'number' ? String(v) : v == null ? '—' : String(v);
}

function ForgePage({ refreshKey }: { refreshKey?: number } = {}) {
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
  }, [refreshKey]);

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
    <div className="overflow-x-hidden">
      <section className="glass mb-6 p-6 sm:p-8">
        <PageToolbar onRefresh={() => void load()} refreshing={loading} />

        {notConfigured ? (
          <EmptyState
            icon={<Inbox size={48} />}
            title="Forge not configured"
            description="Set AETHER_FORGE_URL (and AETHER_FORGE_TOKEN) on the gateway to view GPU capacity, nodes, and placement from the Forge control plane."
          />
        ) : (
          <>
            <StatRibbon
              className="mb-5"
              testId="forge-stats"
              columns={4}
              items={[
                { label: 'Total GPUs', value: num(stats?.totalGPUs), tone: 'white' },
                { label: 'Available', value: num(stats?.availableGPUs), tone: 'emerald' },
                { label: 'Allocated', value: num(stats?.allocatedGPUs), tone: 'amber' },
                { label: 'Utilization %', value: num(stats?.utilizationPercent), tone: 'aether' },
              ]}
            />

            {nodes.length === 0 ? (
              <EmptyState
                icon={<Inbox size={48} />}
                title="No GPU nodes"
                description="Forge reports no GPU-capable nodes on this cluster."
              />
            ) : (
              <CardGrid columns="compact" testId="forge-nodes">
                {nodes.map((n, i) => {
                  const name = n.metadata?.name ?? '—';
                  const gpus = num(n.spec?.gpuCount);
                  return (
                    <EntityCard
                      key={name + i}
                      index={i}
                      testId={`forge-card-${name}`}
                      icon={<Cpu size={18} />}
                      statusTone="sky"
                      title={name}
                      subtitle="GPU node"
                      body={
                        <div className="flex flex-wrap gap-1.5">
                          <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">
                            {gpus} GPU{gpus === '1' ? '' : 's'}
                          </span>
                        </div>
                      }
                    />
                  );
                })}
              </CardGrid>
            )}
          </>
        )}
      </section>
    </div>
  );
}

export default withAuroraPage('forge', ForgePage);