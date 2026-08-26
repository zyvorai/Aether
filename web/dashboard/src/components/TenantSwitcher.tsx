// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { useCallback, useEffect, useState } from 'react';
import { Building2 } from 'lucide-react';
import { apiFetchSettled } from '../utils/api';
import { getActiveTenantId, setActiveTenantId } from '../utils/tenantContext';
import type { HostedTenant } from '../types/api';

export default function TenantSwitcher() {
  const [tenants, setTenants] = useState<HostedTenant[]>([]);
  const [active, setActive] = useState<string>(() => getActiveTenantId() ?? '');

  const load = useCallback(async () => {
    const res = await apiFetchSettled<HostedTenant[]>('/hosted/tenants');
    if (res.ok) setTenants(res.data.filter((t) => t.active));
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const onChange = () => setActive(getActiveTenantId() ?? '');
    window.addEventListener('aether-tenant-changed', onChange);
    return () => window.removeEventListener('aether-tenant-changed', onChange);
  }, []);

  if (tenants.length === 0) return null;

  return (
    <label className="hidden items-center gap-2 text-xs text-ink-2 md:flex" data-testid="navbar-tenant-switcher">
      <Building2 className="h-3.5 w-3.5 shrink-0 text-brand" aria-hidden />
      <span className="sr-only">Active tenant</span>
      <select
        value={active}
        onChange={(e) => {
          const next = e.target.value;
          setActiveTenantId(next || null);
          setActive(next);
        }}
        className="max-w-[9rem] truncate rounded-lg border border-[var(--border)] bg-[var(--surface-elevated)] px-2 py-1 text-xs text-ink"
        data-testid="navbar-tenant-select"
      >
        <option value="">All tenants</option>
        {tenants.map((t) => (
          <option key={t.id} value={t.slug}>
            {t.name}
          </option>
        ))}
      </select>
    </label>
  );
}
