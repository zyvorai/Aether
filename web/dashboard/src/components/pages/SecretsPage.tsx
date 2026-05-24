import { useState, useEffect, useCallback, useMemo, Fragment } from 'react';
import { Inbox, ChevronDown, ChevronRight, Trash2 } from 'lucide-react';
import { apiFetch, apiDelete, apiFetchSettled } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';
import Modal from '../Modal';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { SecretSummary, SecretDetail } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function SecretsPage() {
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useState('');
  const [expandedSecret, setExpandedSecret] = useState<string | null>(null);
  const [secretDetail, setSecretDetail] = useState<SecretDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

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

  if (loading && secrets.length === 0 && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Secrets unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <p className="mb-4 text-sm text-slate-500 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-3">
        Secrets stored with <code className="text-slate-400">VaultRef</code> are external references only — values cannot be decrypted or displayed in this UI.
      </p>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search secrets…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {secrets.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No secrets" description="No secrets have been stored" />
      ) : (
        <div className="dash-card overflow-hidden">
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
                                <span className="text-slate-200">{secretDetail.keys.join(', ')}</span>
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

      <Modal isOpen={confirmDelete !== null} onClose={() => setConfirmDelete(null)} title="Confirm delete">
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
      </Modal>
    </div>
  );
}
