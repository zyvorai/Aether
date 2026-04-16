import { useState, useEffect } from 'react';
import { Inbox, ChevronDown, ChevronRight, Trash2 } from 'lucide-react';
import { apiFetch, apiDelete } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import Badge from '../Badge';
import Modal from '../Modal';
import CodeBlock from '../CodeBlock';
import EmptyState from '../EmptyState';
import type { SecretSummary, SecretDetail } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function SecretsPage() {
  const [secrets, setSecrets] = useState<SecretSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedSecret, setExpandedSecret] = useState<string | null>(null);
  const [secretDetail, setSecretDetail] = useState<SecretDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    const data = await apiFetch<SecretSummary[]>('/secrets');
    setSecrets(data ?? []);
    setLoading(false);
  }

  useEffect(() => { load(); }, []);

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
    const res = await apiDelete(`/secrets/${name}`);
    setConfirmDelete(null);
    if (res.success) {
      toast(`Secret "${name}" deleted`, 'success');
      if (expandedSecret === name) {
        setExpandedSecret(null);
        setSecretDetail(null);
      }
      load();
    } else {
      toast(`Failed to delete secret "${name}": ${res.error ?? 'unknown error'}`, 'error');
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
      </div>
    );
  }

  return (
    <div>
      {secrets.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No secrets" description="No secrets have been stored" />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Namespace</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Keys</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Rotation</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Updated</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {secrets.map((s) => (
                  <>
                    <tr key={`${s.namespace}/${s.name}`} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                      <td className="py-3 px-4">
                        <button
                          onClick={() => handleExpand(s.name)}
                          className="flex items-center gap-1.5 font-medium text-zinc-200 hover:text-amber-400 transition-colors"
                        >
                          {detailLoading === s.name ? (
                            <div className="animate-spin rounded-full h-3 w-3 border-b border-amber-500" />
                          ) : expandedSecret === s.name ? (
                            <ChevronDown size={14} />
                          ) : (
                            <ChevronRight size={14} />
                          )}
                          {s.name}
                        </button>
                      </td>
                      <td className="py-3 px-4 text-sm text-zinc-400">{s.namespace}</td>
                      <td className="py-3 px-4 text-sm text-zinc-300">{s.key_count}</td>
                      <td className="py-3 px-4">
                        <Badge
                          text={s.needs_rotation ? 'Needs Rotation' : 'OK'}
                          variant={s.needs_rotation ? 'red' : 'green'}
                        />
                      </td>
                      <td className="py-3 px-4 text-sm text-zinc-400">{formatTimestamp(s.updated_at)}</td>
                      <td className="py-3 px-4">
                        <button
                          onClick={() => setConfirmDelete(s.name)}
                          className="p-1.5 text-zinc-400 hover:text-red-400 hover:bg-red-500/10 rounded transition-colors"
                          title="Delete"
                        >
                          <Trash2 size={14} />
                        </button>
                      </td>
                    </tr>
                    {expandedSecret === s.name && secretDetail && (
                      <tr key={`${s.namespace}/${s.name}-detail`} className="border-b border-zinc-800/50">
                        <td colSpan={6} className="px-4 py-3">
                          <div className="bg-zinc-950/50 rounded-lg p-4 space-y-3">
                            <div className="text-sm text-zinc-300 font-medium">Secret Detail</div>
                            <div className="grid grid-cols-2 gap-3 text-sm">
                              <div>
                                <span className="text-zinc-500">Keys: </span>
                                <span className="text-zinc-200">{secretDetail.keys.join(', ')}</span>
                              </div>
                              <div>
                                <span className="text-zinc-500">Created: </span>
                                <span className="text-zinc-200">{formatTimestamp(secretDetail.created_at)}</span>
                              </div>
                            </div>
                            {secretDetail.rotation_policy && (
                              <div className="text-sm">
                                <span className="text-zinc-500">Rotation Policy: </span>
                                <span className="text-zinc-300">
                                  Every {secretDetail.rotation_policy.interval_days} days,
                                  max age {secretDetail.rotation_policy.max_age_days} days,
                                  notify {secretDetail.rotation_policy.notify_before_days} days before
                                </span>
                              </div>
                            )}
                            <CodeBlock title="json">{JSON.stringify(secretDetail, null, 2)}</CodeBlock>
                          </div>
                        </td>
                      </tr>
                    )}
                  </>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Delete Confirmation Modal */}
      <Modal
        isOpen={confirmDelete !== null}
        onClose={() => setConfirmDelete(null)}
        title="Confirm Delete"
      >
        <p className="text-sm text-zinc-300 mb-6">
          Are you sure you want to delete secret &quot;{confirmDelete}&quot;? This action cannot be undone.
        </p>
        <div className="flex justify-end gap-3">
          <button
            onClick={() => setConfirmDelete(null)}
            className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-sm font-medium transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => confirmDelete && handleDelete(confirmDelete)}
            className="px-4 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg text-sm font-medium transition-colors"
          >
            Delete
          </button>
        </div>
      </Modal>
    </div>
  );
}
