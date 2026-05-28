// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo, Fragment } from 'react';
import { Inbox, ChevronDown, ChevronRight, Trash2, Plus, Copy } from 'lucide-react';
import { apiFetch, apiDelete, apiFetchSettled, apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import { useNavigate, Link } from 'react-router';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam, useWorkloadOrSearchFilter } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';
import Modal from '../Modal';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { SecretSummary, SecretDetail } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function SecretsPage() {
  const navigate = useNavigate();
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [expandedSecret, setExpandedSecret] = useState<string | null>(null);
  const [secretDetail, setSecretDetail] = useState<SecretDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [createName, setCreateName] = useState('');
  const [createNamespace, setCreateNamespace] = useState('default');
  const [createKeys, setCreateKeys] = useState('API_KEY=\nDATABASE_URL=');
  const [creating, setCreating] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<SecretSummary[]>('/secrets');
    if (!result.ok) {
      setLoadFailed(true);
      setSecrets([]);
    } else {
      setSecrets(result.data);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return secrets;
    return secrets.filter(
      (s) => s.name.toLowerCase().includes(q) || s.namespace.toLowerCase().includes(q)
    );
  }, [secrets, search]);

  async function handleExpand(name: string) {
    if (expandedSecret === name) {
      setExpandedSecret(null);
      setSecretDetail(null);
      return;
    }
    setDetailLoading(name);
    const data = await apiFetch<SecretDetail>(`/secrets/${name}`);
    setSecretDetail(data);
    setExpandedSecret(name);
    setDetailLoading(null);
  }

  async function handleCreate() {
    const keys: Record<string, string> = {};
    for (const line of createKeys.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      const eq = trimmed.indexOf('=');
      if (eq <= 0) continue;
      keys[trimmed.slice(0, eq).trim()] = trimmed.slice(eq + 1);
    }
    if (!createName.trim() || Object.keys(keys).length === 0) {
      toast('Name and at least one KEY=value pair are required', 'error');
      return;
    }
    setCreating(true);
    const res = await apiPost('/secrets', {
      name: createName.trim(),
      namespace: createNamespace.trim() || undefined,
      keys,
    });
    setCreating(false);
    if (res.success) {
      toast(`Secret "${createName.trim()}" created`, 'success');
      setCreateOpen(false);
      setCreateName('');
      setCreateKeys('API_KEY=\nDATABASE_URL=');
      void load();
    } else {
      toast(res.error ?? 'Failed to create secret', 'error');
    }
  }

  async function handleDelete(name: string) {
    const res = await apiDelete(`/secrets/${name}`, { label: `Delete secret "${name}"` });
    setConfirmDelete(null);
    if (res.success) {
      toast(`Secret "${name}" deleted`, 'success');
      if (expandedSecret === name) {
        setExpandedSecret(null);
        setSecretDetail(null);
      }
      void load();
    } else {
      toast(`Failed to delete secret "${name}": ${res.error ?? 'unknown error'}`, 'error');
    }
  }

  async function copyKeyName(key: string) {
    try {
      await navigator.clipboard.writeText(key);
      toast(`Copied key name "${key}"`, 'success');
    } catch {
      toast('Could not copy to clipboard', 'error');
    }
  }

  if (loading && secrets.length === 0 && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Secrets unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <p className="mb-4 text-sm text-slate-500 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-3">
        Secrets stored with <code className="text-slate-400">VaultRef</code> are external references only — values cannot be decrypted or displayed in this UI.{' '}
        <button type="button" onClick={() => navigate(viewToPath('rbac'))} className="text-aether hover:underline">
          API access control →
        </button>
        {' · '}
        <button
          type="button"
          data-testid="secrets-confidential-link"
          onClick={() => navigate(viewToPath('confidential'))}
          className="text-aether hover:underline"
        >
          Confidential computing →
        </button>
      </p>
      {search.trim() ? (
        <div
          data-testid="secrets-workload-context"
          className="mb-4 rounded-xl border border-aether/30 bg-aether/5 px-4 py-3 text-sm text-slate-300"
        >
          Secrets filter <span className="font-mono text-aether">{search.trim()}</span>
          {' · '}
          <button
            type="button"
            onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: search.trim() }))}
            className="text-aether hover:underline"
          >
            Open workload →
          </button>
          <WorkloadScopedCrossLinks workload={search} prefix="secrets" />
        </div>
      ) : null}
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search secrets…"
        onRefresh={() => void load()}
        refreshing={loading}
        actions={
          <button
            type="button"
            data-testid="secrets-create-button"
            onClick={() => setCreateOpen(true)}
            className="inline-flex items-center gap-2 rounded-xl bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500"
          >
            <Plus size={16} />
            Create secret
          </button>
        }
      />

      {secrets.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No secrets" description="No secrets have been stored" />
      ) : (
        <div className="dash-card overflow-hidden" data-testid="secrets-list">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-800">
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Namespace</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Keys</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Rotation</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Updated</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((s) => (
                  <Fragment key={`${s.namespace}/${s.name}`}>
                    <tr className="border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors">
                      <td className="py-3 px-4">
                        <button
                          type="button"
                          onClick={() => void handleExpand(s.name)}
                          className="flex items-center gap-1.5 font-medium text-slate-200 hover:text-aether transition-colors"
                        >
                          {detailLoading === s.name ? (
                            <div className="animate-spin rounded-full h-3 w-3 border-b border-aether" />
                          ) : expandedSecret === s.name ? (
                            <ChevronDown size={14} />
                          ) : (
                            <ChevronRight size={14} />
                          )}
                          {s.name}
                        </button>
                      </td>
                      <td className="py-3 px-4 text-sm text-slate-400">{s.namespace}</td>
                      <td className="py-3 px-4 text-sm text-slate-300">{s.key_count}</td>
                      <td className="py-3 px-4">
                        <Badge
                          text={s.needs_rotation ? 'Needs rotation' : 'OK'}
                          variant={s.needs_rotation ? 'red' : 'green'}
                        />
                      </td>
                      <td className="py-3 px-4 text-sm text-slate-400">{formatTimestamp(s.updated_at)}</td>
                      <td className="py-3 px-4">
                        <button
                          type="button"
                          onClick={() => setConfirmDelete(s.name)}
                          className="p-1.5 text-slate-400 hover:text-red-400 hover:bg-red-500/10 rounded transition-colors"
                          title="Delete"
                        >
                          <Trash2 size={14} />
                        </button>
                      </td>
                    </tr>
                    {expandedSecret === s.name && secretDetail && (
                      <tr className="border-b border-slate-800/50">
                        <td colSpan={6} className="px-4 py-3">
                          <div className="bg-slate-950/50 rounded-xl p-4 space-y-3 border border-slate-800">
                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-sm">
                              <div>
                                <span className="text-slate-500">Keys: </span>
                                <span className="text-slate-200 inline-flex flex-wrap gap-2">
                                  {secretDetail.keys.map((key) => (
                                    <span key={key} className="inline-flex items-center gap-1 rounded border border-slate-800 px-2 py-0.5 font-mono text-xs">
                                      {key}
                                      <button
                                        type="button"
                                        onClick={() => void copyKeyName(key)}
                                        className="text-slate-500 hover:text-aether"
                                        title="Copy key name"
                                        data-testid={`secrets-copy-key-${key}`}
                                      >
                                        <Copy size={12} />
                                      </button>
                                    </span>
                                  ))}
                                </span>
                              </div>
                              <div>
                                <span className="text-slate-500">Created: </span>
                                <span className="text-slate-200">{formatTimestamp(secretDetail.created_at)}</span>
                              </div>
                            </div>
                            {secretDetail.rotation_policy && (
                              <p className="text-sm text-slate-400">
                                Rotation every {secretDetail.rotation_policy.interval_days} days, max age{' '}
                                {secretDetail.rotation_policy.max_age_days} days, notify{' '}
                                {secretDetail.rotation_policy.notify_before_days} days before
                              </p>
                            )}
                          </div>
                        </td>
                      </tr>
                    )}
                  </Fragment>
                ))}
              </tbody>
            </table>
            {filtered.length === 0 && (
              <p className="text-sm text-slate-500 py-6 text-center">No secrets match your search.</p>
            )}
          </div>
        </div>
      )}

      <Modal isOpen={createOpen} onClose={() => setCreateOpen(false)} title="Create secret">
        <div data-testid="secrets-create-modal" className="space-y-4">
          <div>
            <label className="mb-1 block text-xs text-slate-500">Name</label>
            <input
              type="text"
              value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-slate-500">Namespace</label>
            <input
              type="text"
              value={createNamespace}
              onChange={(e) => setCreateNamespace(e.target.value)}
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-slate-500">Keys (KEY=value per line)</label>
            <textarea
              value={createKeys}
              onChange={(e) => setCreateKeys(e.target.value)}
              rows={5}
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 font-mono text-sm text-slate-100"
            />
          </div>
        </div>
        <div className="flex justify-end gap-3 mt-6">
          <button type="button" onClick={() => setCreateOpen(false)} className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-sm">
            Cancel
          </button>
          <button
            type="button"
            disabled={creating}
            onClick={() => void handleCreate()}
            className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-sm font-medium disabled:opacity-50"
          >
            {creating ? 'Creating…' : 'Create'}
          </button>
        </div>
      </Modal>

      <Modal isOpen={confirmDelete !== null} onClose={() => setConfirmDelete(null)} title="Confirm delete">
        <div data-testid="secrets-delete-confirm">
        <p className="text-sm text-slate-300 mb-6">
          Are you sure you want to delete secret &quot;{confirmDelete}&quot;? This cannot be undone.
        </p>
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setConfirmDelete(null)}
            className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-sm font-medium"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => confirmDelete && void handleDelete(confirmDelete)}
            className="px-4 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg text-sm font-medium"
          >
            Delete
          </button>
        </div>
        </div>
      </Modal>
    </div>
  );
}
