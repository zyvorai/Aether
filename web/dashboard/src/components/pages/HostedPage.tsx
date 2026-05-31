// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { useCallback, useEffect, useState } from 'react';
import { Building2, CreditCard, KeyRound, Users } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import type { BillingSummary, HostedTenant } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function HostedPage() {
  const [tenants, setTenants] = useState<HostedTenant[]>([]);
  const [billing, setBilling] = useState<BillingSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [name, setName] = useState('');
  const [slug, setSlug] = useState('');
  const [plan, setPlan] = useState('free');
  const [creating, setCreating] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [tenantsRes, billingRes] = await Promise.all([
      apiFetchSettled<HostedTenant[]>('/hosted/tenants'),
      apiFetchSettled<BillingSummary>('/hosted/billing/usage'),
    ]);
    if (!tenantsRes.ok) {
      setLoadFailed(true);
      setTenants([]);
      setBilling(null);
    } else {
      setTenants(tenantsRes.data);
      setBilling(billingRes.ok ? billingRes.data : null);
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

  if (loading && tenants.length === 0 && !loadFailed) {
    return <PageLoading label="Loading hosted control plane…" />;
  }

  if (loadFailed) {
    return <PageLoadError title="Hosted SaaS unavailable" onRetry={() => void load()} />;
  }

  return (
    <div data-testid="hosted-page">
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

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
        <div className="glass-panel-card-flush overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-[var(--surface-elevated)] text-left">
              <tr>
                <th className="p-3">Name</th>
                <th className="p-3">Slug</th>
                <th className="p-3">Plan</th>
                <th className="p-3">Status</th>
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
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
