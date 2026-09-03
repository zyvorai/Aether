// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Globe, Loader2, Network, RefreshCw, Shield } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { formatUSD } from '../utils/formatters';
import Badge from './Badge';
import type {
  CloudAccountVaultReport,
  ClusterHealthMeshReport,
  CostArbitrageReport,
  FederationExecuteReport,
  GeoPlacementReport,
  PacketWolfGuardReport,
  RegionLockReport,
} from '../types/api';
import GlassSection from './GlassSection';

export default function FederationPlatformPanel() {
  const [tab, setTab] = useState<'mesh' | 'arbitrage' | 'geo' | 'accounts' | 'region' | 'packetwolf'>('mesh');
  const [loading, setLoading] = useState(true);
  const [mesh, setMesh] = useState<ClusterHealthMeshReport | null>(null);
  const [arbitrage, setArbitrage] = useState<CostArbitrageReport | null>(null);
  const [geo, setGeo] = useState<GeoPlacementReport | null>(null);
  const [accounts, setAccounts] = useState<CloudAccountVaultReport | null>(null);
  const [region, setRegion] = useState<RegionLockReport | null>(null);
  const [guard, setGuard] = useState<PacketWolfGuardReport | null>(null);
  const [executing, setExecuting] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const [m, a, g, acc, r, pw] = await Promise.all([
      apiFetch<ClusterHealthMeshReport>('/intelligence/federation/health-mesh'),
      apiFetch<CostArbitrageReport>('/intelligence/multicloud/cost-arbitrage'),
      apiFetch<GeoPlacementReport>('/intelligence/federation/geo-placement'),
      apiFetch<CloudAccountVaultReport>('/intelligence/multicloud/cloud-accounts'),
      apiFetch<RegionLockReport>('/intelligence/federation/region-lock'),
      apiFetch<PacketWolfGuardReport>('/intelligence/federation/packetwolf-guard'),
    ]);
    setMesh(m);
    setArbitrage(a);
    setGeo(g);
    setAccounts(acc);
    setRegion(r);
    setGuard(pw);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function runFederationExecute() {
    setExecuting(true);
    const res = await apiPost<FederationExecuteReport>('/intelligence/federation/execute', { dry_run: true });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: `Federation dry-run → ${res.data?.target_cluster ?? 'none'}`,
            type: 'info',
          },
        }),
      );
    }
  }

  async function applyPacketwolfGuard() {
    setExecuting(true);
    const res = await apiPost<{ applied: string[] }>('/intelligence/federation/packetwolf-guard/apply', {
      dry_run: true,
    });
    setExecuting(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: { message: `PacketWolf guard: ${res.data?.applied.length ?? 0} rules`, type: 'success' },
        }),
      );
    }
  }

  const tabs = [
    { id: 'mesh' as const, label: 'Health mesh' },
    { id: 'arbitrage' as const, label: 'Cost arbitrage' },
    { id: 'geo' as const, label: 'Geo placement' },
    { id: 'accounts' as const, label: 'Cloud accounts' },
    { id: 'region' as const, label: 'Region lock' },
    { id: 'packetwolf' as const, label: 'PacketWolf' },
  ];

  return (
        <GlassSection
      accent="blue"
      testId="federation-platform-panel"
      title="Federation Platform"
      subtitle="Multi-cluster mesh, arbitrage, geo placement, and guards"
      icon={<Network className="h-5 w-5 text-mistblue" />}
      actions={<div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void runFederationExecute()}
            disabled={executing}
            className="rounded-xl border border-mistblue/30 bg-mistblue/10 px-3 py-2 text-xs text-mistblue disabled:opacity-60"
            data-testid="federation-execute-dry-run"
          >
            Dry-run federation sync
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-muted"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>}
    ><div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-mistblue/40 bg-mistblue/10 text-mistblue' : 'glass-divider text-muted'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'mesh' ? (
        <div data-testid="cluster-health-mesh">
          <p className="mb-3 text-xs text-subtle">{mesh?.nodes.length ?? 0} clusters · {mesh?.edges.length ?? 0} peer links</p>
          <ul className="grid gap-2 sm:grid-cols-2">
            {(mesh?.nodes ?? []).map((n) => (
              <li key={n.id} className="rounded-lg border glass-divider px-3 py-2 text-sm">
                <span className="font-medium text-foreground">{n.label}</span>
                <Badge text={n.reachable ? 'up' : 'down'} variant={n.reachable ? 'green' : 'red'} />
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'arbitrage' ? (
        <div data-testid="cost-arbitrage-panel">
          <p className="mb-3 text-sm text-success">
            Potential savings {formatUSD(arbitrage?.total_savings_usd ?? 0)}/mo
          </p>
          <ul className="space-y-2">
            {(arbitrage?.entries ?? []).slice(0, 5).map((e) => (
              <li key={e.workload} className="rounded-lg border glass-divider px-3 py-2 text-sm text-muted">
                {e.workload}: {e.current_provider} → {e.suggested_provider} ({formatUSD(e.savings_usd)}/mo)
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'geo' ? (
        <ul className="space-y-2" data-testid="geo-placement-panel">
          {(geo?.clusters ?? []).slice(0, 6).map((c) => (
            <li key={c.cluster} className="rounded-lg border glass-divider px-3 py-2 text-sm text-muted">
              {c.cluster} · {c.region_hint} · ~{c.estimated_rtt_ms}ms RTT
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'accounts' ? (
        <ul className="space-y-2" data-testid="cloud-accounts-panel">
          {(accounts?.accounts ?? []).map((a) => (
            <li key={a.provider} className="flex items-center gap-2 rounded-lg border glass-divider px-3 py-2 text-sm">
              <Globe className="h-4 w-4 text-muted" />
              <span className="text-foreground">{a.provider}</span>
              <Badge text={a.configured ? 'linked' : 'missing'} variant={a.configured ? 'green' : 'muted'} />
              <span className="text-xs text-subtle">{a.account_hint}</span>
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'region' ? (
        <div data-testid="region-lock-panel">
          <p className="mb-3 text-xs text-subtle">
            Sovereign lock: {region?.sovereign_region_lock ?? 'none configured'}
          </p>
          <ul className="space-y-2">
            {(region?.workloads ?? []).slice(0, 6).map((w) => (
              <li key={w.workload} className="rounded-lg border glass-divider px-3 py-2 text-sm">
                <Shield className="mr-1 inline h-3 w-3" />
                {w.workload}{' '}
                <Badge text={w.compliant ? 'compliant' : 'violation'} variant={w.compliant ? 'green' : 'red'} />
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'packetwolf' ? (
        <div data-testid="packetwolf-guard-panel">
          <button
            type="button"
            onClick={() => void applyPacketwolfGuard()}
            disabled={executing}
            className="mb-4 rounded-xl border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning"
          >
            Apply placement guard (dry-run)
          </button>
          <p className="mb-2 text-xs text-subtle">
            Blocked: {(guard?.blocked_clusters ?? []).join(', ') || 'none'}
          </p>
          <ul className="space-y-2">
            {(guard?.entries ?? []).slice(0, 6).map((e) => (
              <li key={e.cluster} className="rounded-lg border glass-divider px-3 py-2 text-sm text-muted">
                {e.cluster} · {e.anomaly_count} anomalies
                {e.blocked ? <Badge text="blocked" variant="red" /> : null}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </GlassSection>
  );
}
