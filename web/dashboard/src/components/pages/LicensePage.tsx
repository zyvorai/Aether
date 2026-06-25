// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { LicenseStatusResponse, LicenseUsageResponse } from '../../types/api';

type BadgeVariant = 'green' | 'red' | 'yellow' | 'blue' | 'purple' | 'muted' | 'accent';

const STATE_BADGE: Record<string, BadgeVariant> = {
  VALID: 'green',
  EXPIRING_SOON: 'yellow',
  EXPIRED_GRACE: 'yellow',
  OVER_LIMIT_GRACE: 'yellow',
  EXPIRED_BLOCKED: 'red',
  OVER_LIMIT_BLOCKED: 'red',
  MISSING: 'red',
  INVALID_SIGNATURE: 'red',
  WRONG_PRODUCT: 'red',
};

function InfoRow({ label, value }: { label: string; value: string | number | null | undefined }) {
  return (
    <div>
      <div className="text-xs text-zinc-500 uppercase tracking-wide">{label}</div>
      <div className="mt-0.5 text-sm font-medium text-zinc-200">{value ?? '—'}</div>
    </div>
  );
}

function UtilBar({ pct }: { pct: number }) {
  const clamped = Math.min(100, Math.max(0, pct));
  const color = clamped >= 100 ? 'bg-red-500' : clamped >= 80 ? 'bg-amber-400' : 'bg-emerald-500';
  return (
    <div className="h-2 w-full rounded-full bg-zinc-700">
      <div className={`h-2 rounded-full ${color} transition-all`} style={{ width: `${clamped}%` }} />
    </div>
  );
}

export default function LicensePage() {
  const [status, setStatus] = useState<LicenseStatusResponse | null>(null);
  const [usage, setUsage] = useState<LicenseUsageResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [reloadMsg, setReloadMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [s, u] = await Promise.all([
      apiFetchSettled<LicenseStatusResponse>('/license/status'),
      apiFetchSettled<LicenseUsageResponse>('/license/usage'),
    ]);
    if (!s.ok && !u.ok) {
      setLoadFailed(true);
    } else {
      setStatus(s.ok ? s.data : null);
      setUsage(u.ok ? u.data : null);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleReload() {
    setReloading(true);
    setReloadMsg(null);
    const result = await apiPost<{ reloaded: boolean; state: string }>('/license/reload');
    if (result.success && result.data) {
      setReloadMsg(`Reloaded. New state: ${result.data.state}`);
      void load();
    } else {
      setReloadMsg('Reload failed — check server logs.');
    }
    setReloading(false);
  }

  if (loading) return <PageLoading />;
  if (loadFailed)
    return (
      <PageLoadError
        title="License status unavailable"
        description="Could not load license data. Ensure the server is running."
        onRetry={load}
      />
    );

  const state = status?.state ?? 'MISSING';
  const badgeVariant: BadgeVariant = STATE_BADGE[state] ?? 'yellow';
  const allowed = status?.allowed_nodes ?? 0;
  const current = usage?.current_node_count ?? status?.current_node_count ?? 0;
  const utilPct = usage?.utilization_pct ?? (allowed > 0 ? (current / allowed) * 100 : 0);

  return (
    <div className="page-content flex flex-col gap-6">
      <PageToolbar
        actions={
          <button
            type="button"
            onClick={handleReload}
            disabled={reloading}
            className="inline-flex items-center gap-1.5 rounded-lg border border-zinc-600 px-3 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 disabled:opacity-50"
          >
            {reloading ? 'Reloading…' : 'Reload License'}
          </button>
        }
      />

      {reloadMsg && (
        <div className="rounded-lg border border-zinc-700 bg-zinc-800/60 px-4 py-2.5 text-sm text-zinc-300">
          {reloadMsg}
        </div>
      )}

      {status?.warning_message && (
        <div className="rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-200">
          {status.warning_message}
        </div>
      )}

      {/* Summary cards */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <div className="rounded-xl border border-zinc-700 bg-zinc-900/60 px-4 py-3">
          <div className="text-xs text-zinc-500 uppercase tracking-wide mb-1.5">Status</div>
          <Badge text={state.replace(/_/g, ' ')} variant={badgeVariant} />
        </div>
        <div className="rounded-xl border border-zinc-700 bg-zinc-900/60 px-4 py-3">
          <div className="text-xs text-zinc-500 uppercase tracking-wide mb-1">Licensed Nodes</div>
          <div className="text-2xl font-semibold text-zinc-100">{allowed > 0 ? allowed : '—'}</div>
        </div>
        <div className="rounded-xl border border-zinc-700 bg-zinc-900/60 px-4 py-3">
          <div className="text-xs text-zinc-500 uppercase tracking-wide mb-1">Billable Nodes</div>
          <div className="text-2xl font-semibold text-zinc-100">{current}</div>
        </div>
        <div className="rounded-xl border border-zinc-700 bg-zinc-900/60 px-4 py-3">
          <div className="text-xs text-zinc-500 uppercase tracking-wide mb-1">Utilization</div>
          <div className="text-2xl font-semibold text-zinc-100 mb-2">
            {allowed > 0 ? `${Math.round(utilPct)}%` : '—'}
          </div>
          {allowed > 0 && <UtilBar pct={utilPct} />}
        </div>
      </div>

      {/* License details */}
      <div className="rounded-xl border border-zinc-700 bg-zinc-900/60 divide-y divide-zinc-800">
        <div className="px-6 py-4 text-sm font-semibold text-zinc-200">License Details</div>
        {status && (
          <div className="grid grid-cols-2 gap-x-8 gap-y-4 px-6 py-5 sm:grid-cols-3">
            <InfoRow label="License ID" value={status.license_id} />
            <InfoRow label="Customer" value={status.customer} />
            <InfoRow label="Customer ID" value={status.customer_id} />
            <InfoRow label="Product" value={status.product} />
            <InfoRow label="Allowed Nodes" value={status.allowed_nodes} />
            <InfoRow label="Allowed Clusters" value={status.allowed_clusters} />
            <InfoRow label="Valid From" value={status.valid_from} />
            <InfoRow label="Valid Until" value={status.valid_until} />
            <InfoRow label="Days Remaining" value={status.days_remaining} />
            <InfoRow label="Issued At" value={status.issued_at} />
            <InfoRow label="License Version" value={status.license_version} />
          </div>
        )}
      </div>

      {/* Node breakdown table */}
      {usage && usage.nodes.length > 0 && (
        <div className="rounded-xl border border-zinc-700 bg-zinc-900/60">
          <div className="px-6 py-4 text-sm font-semibold text-zinc-200">
            Node Breakdown&ensp;
            <span className="font-normal text-zinc-500">
              {usage.nodes.length} total · {usage.current_node_count} billable
            </span>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="border-t border-zinc-800 bg-zinc-800/40">
                <tr>
                  {['Node', 'Ready', 'Billable'].map((h) => (
                    <th
                      key={h}
                      className="px-6 py-2.5 text-left text-xs font-medium uppercase tracking-wide text-zinc-500"
                    >
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-800">
                {usage.nodes.map((n) => (
                  <tr key={n.name} className="hover:bg-zinc-800/30">
                    <td className="px-6 py-3 font-mono text-xs text-zinc-300">{n.name}</td>
                    <td className="px-6 py-3">
                      <Badge text={n.ready ? 'Ready' : 'NotReady'} variant={n.ready ? 'green' : 'red'} />
                    </td>
                    <td className="px-6 py-3">
                      <Badge text={n.billable ? 'Billable' : 'Excluded'} variant={n.billable ? 'yellow' : 'muted'} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
