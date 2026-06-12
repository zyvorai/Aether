// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

const STORAGE_KEY = 'aether_active_tenant';

/** Active tenant slug/id for multi-tenant hosted API calls (`X-Aether-Tenant`). */
export function getActiveTenantId(): string | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v && v.trim() !== '' ? v.trim() : null;
  } catch {
    return null;
  }
}

export function setActiveTenantId(id: string | null): void {
  try {
    if (!id || id.trim() === '') {
      localStorage.removeItem(STORAGE_KEY);
    } else {
      localStorage.setItem(STORAGE_KEY, id.trim());
    }
    window.dispatchEvent(new CustomEvent('aether-tenant-changed', { detail: { tenantId: id } }));
  } catch {
    /* ignore */
  }
}
