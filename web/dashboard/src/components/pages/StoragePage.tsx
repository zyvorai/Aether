import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { HardDrive, Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatBytes } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import StatRibbon from '../StatRibbon';
import CardGrid from '../CardGrid';
import EntityCard, { type EntityStatusTone } from '../EntityCard';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { StorageVolume, StorageStatus } from '../../types/api';

/** Read an opaque Atlas volume field, tolerating the upstream naming variants. */
function field(vol: StorageVolume, keys: string[]): string {
  for (const k of keys) {
    const v = vol[k];
    if (typeof v === 'string' && v) return v;
  }
  return '—';
}

function stateVariant(state: string): 'green' | 'yellow' | 'red' | 'muted' {
  const s = state.toLowerCase();
  if (s === 'available' || s === 'bound' || s === 'ready') return 'green';
  if (s === 'pending' || s === 'creating') return 'yellow';
  if (s === 'failed' || s === 'error') return 'red';
  return 'muted';
}

function stateTone(state: string): EntityStatusTone {
  const v = stateVariant(state);
  if (v === 'green') return 'green';
  if (v === 'yellow') return 'amber';
  if (v === 'red') return 'red';
  return 'muted';
}

function StoragePage({ refreshKey }: { refreshKey?: number } = {}) {
  const [volumes, setVolumes] = useState<StorageVolume[]>([]);
  const [status, setStatus] = useState<StorageStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [volsRes, statusRes] = await Promise.all([
      apiFetchSettled<StorageVolume[]>('/storage/volumes'),
      apiFetchSettled<StorageStatus>('/storage/status'),
    ]);
    if (!volsRes.ok && !statusRes.ok) {
      setLoadFailed(true);
    } else {
      setVolumes(volsRes.ok ? volsRes.data : []);
      setStatus(statusRes.ok ? statusRes.data : null);
    }
    setLoading(false);
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && volumes.length === 0 && !status && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Storage unavailable" onRetry={() => void load()} />;
  }

  const notConfigured = status !== null && status.configured === false;
  const totalBytes = status?.total_size_bytes ?? 0;

  return (
    <div className="overflow-x-hidden">
      <section className="glass mb-6 p-6 sm:p-8">
        <PageToolbar onRefresh={() => void load()} refreshing={loading} />

        {notConfigured ? (
          <EmptyState
            icon={<Inbox size={48} />}
            title="Atlas storage not configured"
            description="Set AETHER_ATLAS_URL (and AETHER_ATLAS_TOKEN) on the gateway to provision and view Atlas-backed volumes."
          />
        ) : (
          <>
            <StatRibbon
              className="mb-5"
              testId="storage-status"
              columns={4}
              items={[
                { label: 'Volumes', value: status?.volumes ?? volumes.length, tone: 'white' },
                { label: 'Total size', value: formatBytes(totalBytes), tone: 'sky' },
                { label: 'Tenant', value: status?.tenant ?? '—', tone: 'violet' },
                { label: 'Endpoint', value: status?.endpoint ?? '—', tone: 'white' },
              ]}
            />

            {volumes.length === 0 ? (
              <EmptyState
                icon={<Inbox size={48} />}
                title="No volumes"
                description="No Atlas volumes are owned by this tenant yet. Deploy a workload with storageClass: atlas/<policy> to provision one."
              />
            ) : (
              <CardGrid columns="compact" testId="storage-volumes">
                {volumes.map((v, i) => {
                  const state = field(v, ['state']);
                  const name = field(v, ['name']);
                  return (
                    <EntityCard
                      key={field(v, ['id', 'volume_id']) + i}
                      index={i}
                      testId={`storage-card-${name}`}
                      icon={<HardDrive size={18} />}
                      statusTone={stateTone(state)}
                      pulse={stateVariant(state) === 'green'}
                      title={name}
                      subtitle={[field(v, ['namespace', 'kubernetes_namespace']), field(v, ['storage_class', 'storage_class_name'])]
                        .filter((x) => x !== '—')
                        .join(' · ') || 'Atlas volume'}
                      badge={state === '—' ? undefined : <Badge text={state} variant={stateVariant(state)} />}
                      body={
                        <div className="flex flex-wrap gap-1.5">
                          <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">
                            PVC {field(v, ['pvc', 'pvc_name'])}
                          </span>
                          <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-muted">
                            {typeof v.size_bytes === 'number' ? formatBytes(v.size_bytes) : '—'}
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

export default withAuroraPage('storage', StoragePage);