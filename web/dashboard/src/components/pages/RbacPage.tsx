import { useEffect, useState } from 'react';
import { KeyRound, Shield, Trash2 } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import Modal from '../Modal';
import type { ApiKeySummary, CreateApiKeyResponse } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function RbacPage() {
  const [keys, setKeys] = useState<ApiKeySummary[]>([]);
  const [name, setName] = useState('');
  const [role, setRole] = useState('viewer');
  const [loading, setLoading] = useState(false);
  const [created, setCreated] = useState<CreateApiKeyResponse | null>(null);
  const [revokeName, setRevokeName] = useState<string | null>(null);

  async function load() {
    const data = await apiFetch<ApiKeySummary[]>('/rbac/keys');
    setKeys(data ?? []);
  }

  useEffect(() => {
    load();
  }, []);

  async function handleCreate() {
    if (!name.trim()) return;
    setLoading(true);
    const response = await apiPost<CreateApiKeyResponse>('/rbac/keys', { name: name.trim(), role });
    setLoading(false);
    if (response.success && response.data) {
      setCreated(response.data);
      setName('');
      setRole('viewer');
      toast(`API key "${response.data.name}" created`, 'success');
      load();
    } else {
      toast(response.error ?? 'Failed to create API key', 'error');
    }
  }

  async function handleRevoke() {
    if (!revokeName) return;
    const response = await apiPost('/rbac/keys/revoke', { name: revokeName });
    if (response.success) {
      toast(`API key "${revokeName}" revoked`, 'success');
      setRevokeName(null);
      load();
    } else {
      toast(response.error ?? 'Failed to revoke API key', 'error');
    }
  }

  return (
    <div>
      <div className="rounded-2xl surface-panel p-6 mb-6">
        <div className="flex items-center gap-3 mb-4">
          <Shield className="w-5 h-5 text-aether" />
          <h2 className="text-lg font-semibold text-slate-100">Create RBAC API Key</h2>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-[1.4fr_0.8fr_auto] gap-3">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Key name, e.g. ci-bot or ops-viewer"
            className="rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-500"
          />
          <select
            value={role}
            onChange={(e) => setRole(e.target.value)}
            className="rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2.5 text-sm text-slate-100"
          >
            <option value="admin">Admin</option>
            <option value="operator">Operator</option>
            <option value="viewer">Viewer</option>
          </select>
          <button
            onClick={handleCreate}
            disabled={loading || !name.trim()}
            className="rounded-xl bg-aether hover:bg-aether-light disabled:opacity-50 px-4 py-2.5 text-sm font-medium text-white transition-colors"
          >
            {loading ? 'Creating...' : 'Create Key'}
          </button>
        </div>
      </div>

      {keys.length === 0 ? (
        <EmptyState icon={<KeyRound size={48} />} title="No RBAC keys" description="Create admin, operator, or viewer API keys to expose role-based access." />
      ) : (
        <div className="rounded-2xl surface-panel overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-800">
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Role</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Created</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {keys.map((entry) => (
                  <tr key={entry.name} className="border-b border-slate-800/70 hover:bg-slate-900/40">
                    <td className="py-3 px-4 text-sm font-medium text-slate-100">{entry.name}</td>
                    <td className="py-3 px-4">
                      <Badge
                        text={entry.role}
                        variant={entry.role === 'admin' ? 'red' : entry.role === 'operator' ? 'yellow' : 'blue'}
                      />
                    </td>
                    <td className="py-3 px-4 text-sm text-slate-400">{entry.created_at}</td>
                    <td className="py-3 px-4">
                      <button
                        onClick={() => setRevokeName(entry.name)}
                        className="inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm text-red-300 hover:bg-red-500/10"
                      >
                        <Trash2 size={14} />
                        Revoke
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal isOpen={created !== null} onClose={() => setCreated(null)} title={`New API Key: ${created?.name ?? ''}`}>
        {created && (
          <div className="space-y-4">
            <p className="text-sm text-slate-400">This plaintext key is only returned once. Store it before closing.</p>
            <div className="rounded-xl border border-aether/20 bg-slate-950/80 px-4 py-3 font-mono text-sm text-aether break-all">
              {created.key}
            </div>
            <Badge text={created.role} variant={created.role === 'admin' ? 'red' : created.role === 'operator' ? 'yellow' : 'blue'} />
          </div>
        )}
      </Modal>

      <Modal isOpen={revokeName !== null} onClose={() => setRevokeName(null)} title="Revoke API Key">
        <div className="space-y-4">
          <p className="text-sm text-slate-400">Revoke access for <span className="text-slate-200 font-medium">{revokeName}</span>?</p>
          <div className="flex justify-end gap-3">
            <button onClick={() => setRevokeName(null)} className="rounded-lg px-4 py-2 text-sm text-slate-300 hover:bg-slate-800">Cancel</button>
            <button onClick={handleRevoke} className="rounded-lg bg-red-600 hover:bg-red-500 px-4 py-2 text-sm font-medium text-white">Revoke</button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
