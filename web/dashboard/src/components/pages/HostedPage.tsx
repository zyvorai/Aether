// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Building2, CreditCard, Globe2, KeyRound, Rocket, Users } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { setActiveTenantId } from '../../utils/tenantContext';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import type { BillingSummary, FederationPlan, HostedFederationStatus, HostedTenant } from '../../types/api';

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

export default function HostedPage({ refreshKey }: { refreshKey?: number } = {}) {
  const [tenants, setTenants] = useState<HostedTenant[]>([]);
  const [billing, setBilling] = useState<BillingSummary | null>(null);
  const [upgrades, setUpgrades] = useState<UpgradesStatus | null>(null);
  const [federation, setFederation] = useState<HostedFederationStatus | null>(null);
  const [federationPlan, setFederationPlan] = useState<FederationPlan | null>(null);
  const [planningFederation, setPlanningFederation] = useState(false);
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
    const [tenantsRes, billingRes, upgradesRes, federationRes] = await Promise.all([
      apiFetchSettled<HostedTenant[]>('/hosted/tenants'),
      apiFetchSettled<BillingSummary>('/hosted/billing/usage'),
      apiFetchSettled<UpgradesStatus>('/hosted/upgrades'),
      apiFetchSettled<HostedFederationStatus>('/hosted/federation'),
    ]);
    if (!tenantsRes.ok) {
      setLoadFailed(true);
      setTenants([]);
      setBilling(null);
      setUpgrades(null);
      setFederation(null);
    } else {
      setTenants(tenantsRes.data);
      setBilling(billingRes.ok ? billingRes.data : null);
      setUpgrades(upgradesRes.ok ? upgradesRes.data : null);
      setFederation(federationRes.ok ? federationRes.data : null);
    }
    setLoading(false);
  }, [refreshKey]);

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

  async function handleFederationPlan(tenant: HostedTenant) {
    setPlanningFederation(true);
    const res = await apiPost<FederationPlan>(`/hosted/tenants/${tenant.id}/federation/plan`, {
      workload_name: 'nginx',
    });
    setPlanningFederation(false);
    if (res.success && res.data) {
      setFederationPlan(res.data);
      toast(`Federation plan for ${tenant.slug}: ${res.data.recommended_cluster ?? 'no cluster'}`, 'success');
    } else {
      toast(res.error ?? 'Federation plan failed', 'error');
    }
  }

  async function handleStripeCheckout(tenant: HostedTenant) {
    const nextPlan = tenant.plan === 'free' ? 'team' : 'enterprise';
    const origin = window.location.origin;
    const res = await apiPost<{ checkout_url?: string }>('/hosted/billing/stripe/checkout', {
      tenant_id: tenant.id,
      plan: nextPlan,
      success_url: `${origin}/hosted?billing=success`,
      cancel_url: `${origin}/hosted?billing=cancel`,
    });
    if (res.success && res.data?.checkout_url) {
      window.location.assign(res.data.checkout_url);
    } else {
      toast(res.error ?? 'Stripe checkout unavailable', 'error');
    }
  }

  async function handleStripePortal(tenant: HostedTenant) {
    const res = await apiPost<{ portal_url?: string }>('/hosted/billing/stripe/portal', {
      tenant_id: tenant.id,
      return_url: `${window.location.origin}/hosted`,
    });
    if (res.success && res.data?.portal_url) {
      window.location.assign(res.data.portal_url);
    } else {
      toast(res.error ?? 'Billing portal unavailable — complete checkout first', 'error');
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
        {' · '}
        <Link to={viewToPath('fleet')} className="text-aether hover:underline" data-testid="hosted-context-federation-link">
          Federation →
        </Link>
        {' · '}
        <Link to={viewToPath('settings')} className="text-aether hover:underline" data-testid="hosted-context-settings-link">
          Identity & SSO →
        </Link>
      </div>

      {federation && (
        <section className="overview-section-shell mb-6 p-6 sm:p-8" data-testid="hosted-federation-panel">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <p className="section-label">Managed federation</p>
              <h2 className="section-title flex items-center gap-2">
                <Globe2 size={18} className="text-aether" />
                {federation.federation_enabled ? 'Federation active' : 'Single-cluster mode'}
              </h2>
              <p className="section-subtitle">
                {federation.tenant_count} tenant(s) · {federation.policy.clusters.length} configured cluster(s)
              </p>
            </div>
            {tenants[0] && federation.federation_enabled ? (
              <button
                type="button"
                data-testid="hosted-federation-plan-button"
                disabled={planningFederation}
                onClick={() => void handleFederationPlan(tenants[0])}
                className="btn-secondary disabled:opacity-50"
              >
                {planningFederation ? 'Planning…' : 'Plan tenant placement'}
              </button>
            ) : null}
          </div>
          {federation.policy.clusters.length > 0 ? (
            <div className="mt-4 flex flex-wrap gap-2">
              {federation.policy.clusters.map((cluster) => (
                <Badge
                  key={cluster}
                  variant="blue"
                  text={`${cluster}${federation.policy.weights[cluster] ? ` (${federation.policy.weights[cluster]})` : ''}`}
                />
              ))}
            </div>
          ) : (
            <p className="mt-4 text-sm text-slate-500">Set AETHER_FEDERATION_CLUSTERS to enable multi-cluster placement.</p>
          )}
          {federationPlan && (
            <p className="mt-4 text-sm text-slate-300" data-testid="hosted-federation-plan-result">
              Recommended: {federationPlan.recommended_cluster ?? 'none'} ({federationPlan.recommended_runtime})
            </p>
          )}
        </section>
      )}

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
        <section className="overview-section-shell mb-6 grid grid-cols-1 sm:grid-cols-3 gap-4 p-6 sm:p-8" data-testid="hosted-billing-panel">
          <div className="glass-panel-card py-3 px-4">
            <div className="flex items-center gap-2 text-sm text-slate-500 mb-1">
              <Users size={16} /> Tenants
            </div>
            {/* billing.tenants always has >=1 entry (a synthetic "self-hosted" line when no
                real tenants exist, for usage metering) — using the real tenant count here
                keeps this stat consistent with the Tenants list rendered below. */}
            <div className="text-2xl font-semibold text-slate-100">{tenants.length}</div>
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
        <CardGrid columns="compact" testId="hosted-tenants-list">
          {tenants.map((t, i) => (
            <EntityCard
              key={t.id}
              index={i}
              testId={`hosted-tenant-${t.slug}`}
              icon={<Building2 size={18} />}
              statusTone={t.active ? 'green' : 'muted'}
              pulse={t.active}
              title={t.name}
              subtitle={t.slug}
              badge={<Badge variant={t.active ? 'green' : 'muted'} text={t.active ? 'Active' : 'Inactive'} />}
              body={
                <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] capitalize text-slate-300">
                  Plan: {t.plan}
                </span>
              }
              footer={
                <>
                  <button
                    type="button"
                    className="inline-flex flex-1 items-center justify-center gap-1 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-white/5 hover:text-white"
                    onClick={() => {
                      setActiveTenantId(t.slug);
                      toast(`Active tenant: ${t.slug}`, 'success');
                    }}
                  >
                    Use
                  </button>
                  {t.plan !== 'enterprise' && t.active ? (
                    <button
                      type="button"
                      data-testid="hosted-upgrade-button"
                      disabled={upgradingId === t.id}
                      onClick={() => void handleUpgrade(t)}
                      className="inline-flex flex-1 items-center justify-center gap-1 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-aether/15 hover:text-aether disabled:opacity-50"
                    >
                      {upgradingId === t.id ? '…' : 'Upgrade'}
                    </button>
                  ) : null}
                  {billing?.stripe_configured && t.active && t.plan !== 'enterprise' ? (
                    <button
                      type="button"
                      data-testid="hosted-stripe-checkout-button"
                      onClick={() => void handleStripeCheckout(t)}
                      className="inline-flex flex-1 items-center justify-center gap-1 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-white/5 hover:text-white"
                    >
                      Stripe
                    </button>
                  ) : null}
                  {billing?.stripe_configured && t.active ? (
                    <button
                      type="button"
                      data-testid="hosted-stripe-portal-button"
                      onClick={() => void handleStripePortal(t)}
                      className="inline-flex flex-1 items-center justify-center gap-1 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-white/5 hover:text-white"
                    >
                      Portal
                    </button>
                  ) : null}
                </>
              }
            />
          ))}
        </CardGrid>
      )}
    </div>
  );
}
