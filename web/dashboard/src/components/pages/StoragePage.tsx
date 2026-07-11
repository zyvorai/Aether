// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Inbox } from 'lucide-react';
import { apiFetchSettled } from '../../utils/api';
import { formatBytes } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
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

export default function StoragePage() {
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
  }, []);

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
    <div>
      <section className="overview-section-shell mb-6 p-6 sm:p-8">
        <PageToolbar onRefresh={() => void load()} refreshing={loading} />

        {notConfigured ? (
          <EmptyState
            icon={<Inbox size={48} />}
            title="Atlas storage not configured"
            description="Set AETHER_ATLAS_URL (and AETHER_ATLAS_TOKEN) on the gateway to provision and view Atlas-backed volumes."
          />
        ) : (
          <>
            <div className="mb-6 grid grid-cols-2 gap-3 md:grid-cols-4" data-testid="storage-status">
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Volumes</div>
                <div className="mt-2 text-lg font-semibold text-slate-100">
                  {status?.volumes ?? volumes.length}
                </div>
              </div>
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Total size</div>
                <div className="mt-2 text-lg font-semibold text-slate-100">{formatBytes(totalBytes)}</div>
              </div>
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Tenant</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{status?.tenant ?? '—'}</div>
              </div>
              <div className="glass-panel-card px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Endpoint</div>
                <div className="mt-2 truncate text-sm font-medium text-slate-100" title={status?.endpoint}>
                  {status?.endpoint ?? '—'}
                </div>
              </div>
            </div>

            {volumes.length === 0 ? (
              <EmptyState
                icon={<Inbox size={48} />}
                title="No volumes"
                description="No Atlas volumes are owned by this tenant yet. Deploy a workload with storageClass: atlas/<policy> to provision one."
              />
            ) : (
              <div className="glass-panel-card overflow-hidden" data-testid="storage-volumes">
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="glass-divider-b">
                        <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Name</th>
                        <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">State</th>
                        <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">PVC</th>
                        <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Class</th>
                        <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Namespace</th>
                        <th className="text-right text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Size</th>
                      </tr>
                    </thead>
                    <tbody>
                      {volumes.map((v, i) => {
                        const state = field(v, ['state']);
                        return (
                          <tr
                            key={field(v, ['id', 'volume_id']) + i}
                            className="glass-table-row glass-inset-hover transition-colors"
                          >
                            <td className="py-3 px-4 font-medium text-slate-200">{field(v, ['name'])}</td>
                            <td className="py-3 px-4">
                              {state === '—' ? (
                                <span className="text-sm text-slate-400">—</span>
                              ) : (
                                <Badge text={state} variant={stateVariant(state)} />
                              )}
                            </td>
                            <td className="py-3 px-4 text-sm text-slate-300">{field(v, ['pvc', 'pvc_name'])}</td>
                            <td className="py-3 px-4 text-sm text-slate-300">
                              {field(v, ['storage_class', 'storage_class_name'])}
                            </td>
                            <td className="py-3 px-4 text-sm text-slate-300">
                              {field(v, ['namespace', 'kubernetes_namespace'])}
                            </td>
                            <td className="py-3 px-4 text-right text-sm text-slate-300">
                              {typeof v.size_bytes === 'number' ? formatBytes(v.size_bytes) : '—'}
                            </td>
                          </tr>
                        );
                      })}
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
