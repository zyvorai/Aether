// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Inbox, ChevronDown, ChevronRight, Trash2, Plus, Copy, KeyRound } from 'lucide-react';
import { apiFetch, apiDelete, apiFetchSettled, apiPost } from '../../utils/api';
import { copyToClipboard } from '../../utils/clipboard';
import { formatTimestamp } from '../../utils/formatters';
import { useNavigate, Link } from 'react-router';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useWorkloadOrSearchFilter } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import Badge from '../Badge';
import Modal from '../Modal';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { SearchQueryContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
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
      if (!(await copyToClipboard(key))) throw new Error('clipboard unavailable');
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
      <p className="glass-context-banner mb-4 text-sm text-slate-500">
        Secrets stored with <code className="text-slate-400">VaultRef</code> are external references only — values cannot be decrypted or displayed in this UI.{' '}
        <button
          type="button"
          data-testid="secrets-rbac-link"
          onClick={() =>
            navigate(
              search.trim()
                ? pathWithQuery(viewToPath('rbac'), { workload: search.trim() })
                : viewToPath('rbac'),
            )
          }
          className="text-aether hover:underline"
        >
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
      <SearchQueryContextBanner testId="secrets-workload-context" query={search} entityLabel="secrets">
        <WorkloadScopedCrossLinks workload={search} prefix="secrets" showDrift showAudit />
        {search.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="secrets-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('backups'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="secrets-backups-link"
            >
              Backups →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('rbac'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="secrets-context-rbac-link"
            >
              RBAC →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="secrets-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('confidential'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="secrets-context-confidential-link"
            >
              Confidential →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('gitops'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="secrets-context-gitops-link"
            >
              GitOps →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('templates'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="secrets-context-templates-link"
            >
              Templates →
            </Link>
          </>
        ) : null}
      </SearchQueryContextBanner>

      <section className="overview-section-shell mb-6 p-6 sm:p-8">
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
            className="inline-flex items-center gap-2 btn-primary"
          >
            <Plus size={16} />
            Create secret
          </button>
        }
      />

      {secrets.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No secrets" description="No secrets have been stored" />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No matching secrets" description="Try adjusting your search." />
      ) : (
        <CardGrid columns="compact" testId="secrets-list">
          {filtered.map((s, i) => {
            const expanded = expandedSecret === s.name;
            return (
              <EntityCard
                key={`${s.namespace}/${s.name}`}
                index={i}
                testId={`secret-card-${s.name}`}
                icon={<KeyRound size={18} />}
                statusTone={s.needs_rotation ? 'red' : 'green'}
                title={s.name}
                subtitle={s.namespace}
                badge={<Badge text={s.needs_rotation ? 'Rotate' : 'OK'} variant={s.needs_rotation ? 'red' : 'green'} />}
                body={
                  <>
                    <div className="flex flex-wrap gap-1.5">
                      <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-slate-300">{s.key_count} keys</span>
                      <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-slate-400">{formatTimestamp(s.updated_at)}</span>
                    </div>
                    {expanded && secretDetail ? (
                      <div className="mt-3 space-y-2 rounded-lg border glass-divider glass-inset-surface p-3">
                        <div className="flex flex-wrap gap-1.5">
                          {secretDetail.keys.map((key) => (
                            <span key={key} className="inline-flex items-center gap-1 rounded border glass-divider px-2 py-0.5 font-mono text-[11px] text-slate-300">
                              {key}
                              <button
                                type="button"
                                onClick={() => void copyKeyName(key)}
                                className="text-slate-500 hover:text-aether"
                                title="Copy key name"
                                data-testid={`secrets-copy-key-${key}`}
                              >
                                <Copy size={11} />
                              </button>
                            </span>
                          ))}
                        </div>
                        <p className="text-[11px] text-slate-500">Created {formatTimestamp(secretDetail.created_at)}</p>
                        {secretDetail.rotation_policy ? (
                          <p className="text-[11px] text-slate-400">
                            Rotate every {secretDetail.rotation_policy.interval_days}d · max age {secretDetail.rotation_policy.max_age_days}d · notify {secretDetail.rotation_policy.notify_before_days}d before
                          </p>
                        ) : null}
                      </div>
                    ) : null}
                  </>
                }
                footer={
                  <>
                    <button
                      type="button"
                      onClick={() => void handleExpand(s.name)}
                      className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-white/5 hover:text-white"
                    >
                      {detailLoading === s.name ? (
                        <span className="h-3 w-3 animate-spin rounded-full border-b border-aether" />
                      ) : expanded ? (
                        <ChevronDown size={13} />
                      ) : (
                        <ChevronRight size={13} />
                      )}
                      {expanded ? 'Hide' : 'Keys'}
                    </button>
                    <button
                      type="button"
                      onClick={() => setConfirmDelete(s.name)}
                      className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-red-500/15 hover:text-red-300"
                    >
                      <Trash2 size={13} />
                      Delete
                    </button>
                  </>
                }
              />
            );
          })}
        </CardGrid>
      )}
      </section>


      <Modal isOpen={createOpen} onClose={() => setCreateOpen(false)} title="Create secret">
        <div data-testid="secrets-create-modal" className="space-y-4">
          <div>
            <label className="mb-1 block text-xs text-slate-500">Name</label>
            <input
              type="text"
              value={createName}
              onChange={(e) => setCreateName(e.target.value)}
              className="glass-input"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-slate-500">Namespace</label>
            <input
              type="text"
              value={createNamespace}
              onChange={(e) => setCreateNamespace(e.target.value)}
              className="glass-input"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-slate-500">Keys (KEY=value per line)</label>
            <textarea
              value={createKeys}
              onChange={(e) => setCreateKeys(e.target.value)}
              rows={5}
              className="glass-input font-mono"
            />
          </div>
        </div>
        <div className="flex justify-end gap-3 mt-6">
          <button type="button" onClick={() => setCreateOpen(false)} className="px-4 py-2 glass-inset-surface glass-inset-hover text-slate-200 rounded-lg text-sm">
            Cancel
          </button>
          <button
            type="button"
            disabled={creating}
            onClick={() => void handleCreate()}
            className="btn-primary disabled:opacity-50"
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
            className="px-4 py-2 glass-inset-surface glass-inset-hover text-slate-200 rounded-lg text-sm font-medium"
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
