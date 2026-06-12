// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Building2, CreditCard, KeyRound, Rocket, Users } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { setActiveTenantId } from '../../utils/tenantContext';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import type { BillingSummary, HostedTenant } from '../../types/api';

interface UpgradesStatus {
  current_version: string;
  channel: string;
  managed_upgrades: boolean;
  upgrade_available: boolean;
  notes: string;
}

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function HostedPage() {
  const [tenants, setTenants] = useState<HostedTenant[]>([]);
  const [billing, setBilling] = useState<BillingSummary | null>(null);
  const [upgrades, setUpgrades] = useState<UpgradesStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [name, setName] = useState('');
  const [slug, setSlug] = useState('');
  const [plan, setPlan] = useState('free');
  const [creating, setCreating] = useState(false);
  const [upgradingId, setUpgradingId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [tenantsRes, billingRes, upgradesRes] = await Promise.all([
      apiFetchSettled<HostedTenant[]>('/hosted/tenants'),
      apiFetchSettled<BillingSummary>('/hosted/billing/usage'),
      apiFetchSettled<UpgradesStatus>('/hosted/upgrades'),
    ]);
    if (!tenantsRes.ok) {
      setLoadFailed(true);
      setTenants([]);
      setBilling(null);
      setUpgrades(null);
    } else {
      setTenants(tenantsRes.data);
      setBilling(billingRes.ok ? billingRes.data : null);
      setUpgrades(upgradesRes.ok ? upgradesRes.data : null);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleCreate() {
    if (!name.trim() || !slug.trim()) return;
    setCreating(true);
    const res = await apiPost<HostedTenant>('/hosted/tenants', {
      name: name.trim(),
      slug: slug.trim(),
      plan,
    });
    setCreating(false);
    if (res.success) {
      toast(`Tenant "${slug}" created`, 'success');
      setName('');
      setSlug('');
      void load();
    } else {
      toast(res.error ?? 'Failed to create tenant', 'error');
    }
  }

  async function handleUpgrade(tenant: HostedTenant) {
    const nextPlan = tenant.plan === 'free' ? 'team' : tenant.plan === 'team' ? 'enterprise' : 'enterprise';
    setUpgradingId(tenant.id);
    const res = await apiPost<HostedTenant>(`/hosted/tenants/${tenant.id}/upgrade`, { plan: nextPlan });
    setUpgradingId(null);
    if (res.success) {
      toast(`Tenant "${tenant.slug}" upgraded to ${nextPlan}`, 'success');
      void load();
    } else {
      toast(res.error ?? 'Upgrade failed', 'error');
    }
  }

  if (loading && tenants.length === 0 && !loadFailed) {
    return <PageLoading label="Loading hosted control plane…" />;
  }

  if (loadFailed) {
    return <PageLoadError title="Hosted SaaS unavailable" onRetry={() => void load()} />;
  }

  return (
    <div data-testid="hosted-page">
      <div className="mb-6 glass-context-banner" data-testid="hosted-hub-context">
        Hosted control plane
        {' · '}
        <Link to={viewToPath('fleet')} className="text-aether hover:underline" data-testid="hosted-context-fleet-link">
          Fleet →
        </Link>
        {' · '}
        <Link to={viewToPath('platform')} className="text-aether hover:underline" data-testid="hosted-context-platform-link">
          Platform →
        </Link>
      </div>

      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

      {upgrades && (
        <section className="overview-section-shell mb-6 p-6 sm:p-8" data-testid="hosted-upgrades-panel">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <p className="section-label">Managed upgrades</p>
              <h2 className="section-title">Version {upgrades.current_version}</h2>
              <p className="section-subtitle">{upgrades.notes}</p>
            </div>
            <div className="flex items-center gap-2 text-sm text-slate-400">
              <Rocket size={16} className="text-aether" />
              Channel: {upgrades.channel}
              {upgrades.upgrade_available ? (
                <Badge variant="yellow" text="Upgrade available" />
              ) : (
                <Badge variant="green" text="Up to date" />
              )}
            </div>
          </div>
        </section>
      )}

      {billing && (
        <section className="overview-section-shell mb-6 grid grid-cols-1 sm:grid-cols-3 gap-4 p-6 sm:p-8">
          <div className="glass-panel-card py-3 px-4">
            <div className="flex items-center gap-2 text-sm text-slate-500 mb-1">
              <Users size={16} /> Tenants
            </div>
            <div className="text-2xl font-semibold text-slate-100">{billing.tenants.length}</div>
          </div>
          <div className="glass-panel-card py-3 px-4">
            <div className="flex items-center gap-2 text-sm text-slate-500 mb-1">
              <Building2 size={16} /> Workloads
            </div>
            <div className="text-2xl font-semibold text-slate-100">{billing.total_workloads}</div>
          </div>
          <div className="glass-panel-card py-3 px-4">
            <div className="flex items-center gap-2 text-sm text-slate-500 mb-1">
              <CreditCard size={16} /> Period
            </div>
            <div className="text-2xl font-semibold text-slate-100">{billing.period}</div>
          </div>
        </section>
      )}

      <div className="glass-panel-card mb-6">
        <h3 className="font-medium mb-3 flex items-center gap-2">
          <KeyRound size={18} /> Create tenant
        </h3>
        <div className="flex flex-wrap gap-3 items-end">
          <label className="text-sm">
            Name
            <input
              className="block mt-1 rounded border border-[var(--border)] bg-[var(--bg)] px-3 py-2"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <label className="text-sm">
            Slug
            <input
              className="block mt-1 rounded border border-[var(--border)] bg-[var(--bg)] px-3 py-2"
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
            />
          </label>
          <label className="text-sm">
            Plan
            <select
              className="block mt-1 rounded border border-[var(--border)] bg-[var(--bg)] px-3 py-2"
              value={plan}
              onChange={(e) => setPlan(e.target.value)}
            >
              <option value="free">Free</option>
              <option value="team">Team</option>
              <option value="enterprise">Enterprise</option>
            </select>
          </label>
          <button
            type="button"
            disabled={creating}
            onClick={() => void handleCreate()}
            className="btn-primary disabled:opacity-50"
          >
            {creating ? 'Creating…' : 'Create'}
          </button>
        </div>
      </div>

      {tenants.length === 0 ? (
        <EmptyState
          icon={<Users size={32} />}
          title="No tenants"
          description="Create a tenant to enable multi-tenant API keys and billing."
        />
      ) : (
        <div className="glass-table-shell overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-[var(--surface-elevated)] text-left">
              <tr>
                <th className="p-3">Name</th>
                <th className="p-3">Slug</th>
                <th className="p-3">Plan</th>
                <th className="p-3">Status</th>
                <th className="p-3">Actions</th>
              </tr>
            </thead>
            <tbody>
              {tenants.map((t) => (
                <tr key={t.id} className="border-t border-[var(--border)]">
                  <td className="p-3">{t.name}</td>
                  <td className="p-3 font-mono text-xs">{t.slug}</td>
                  <td className="p-3">{t.plan}</td>
                  <td className="p-3">
                    <Badge variant={t.active ? 'green' : 'muted'} text={t.active ? 'Active' : 'Inactive'} />
                  </td>
                  <td className="p-3 flex flex-wrap gap-2">
                    <button
                      type="button"
                      className="btn-secondary text-xs"
                      onClick={() => {
                        setActiveTenantId(t.slug);
                        toast(`Active tenant: ${t.slug}`, 'success');
                      }}
                    >
                      Use tenant
                    </button>
                    {t.plan !== 'enterprise' && t.active ? (
                      <button
                        type="button"
                        data-testid="hosted-upgrade-button"
                        disabled={upgradingId === t.id}
                        onClick={() => void handleUpgrade(t)}
                        className="btn-primary text-xs disabled:opacity-50"
                      >
                        {upgradingId === t.id ? 'Upgrading…' : 'Upgrade plan'}
                      </button>
                    ) : null}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
