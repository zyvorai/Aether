// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState, useCallback, useMemo } from 'react';
import { Link } from 'react-router';
import { KeyRound, Shield, Trash2, Copy } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { useQueryParam } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import Modal from '../Modal';
import { useAuth } from '../../contexts/AuthContext';
import type { ApiKeySummary, CreateApiKeyResponse } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function RbacPage() {
  const { canAdmin } = useAuth();
  const [keys, setKeys] = useState<ApiKeySummary[]>([]);
  const [name, setName] = useState('');
  const [role, setRole] = useState('viewer');
  const [loading, setLoading] = useState(false);
  const [listLoading, setListLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useQueryParam('q');
  const [created, setCreated] = useState<CreateApiKeyResponse | null>(null);
  const [keyCopied, setKeyCopied] = useState(false);
  const [revokeName, setRevokeName] = useState<string | null>(null);

  const load = useCallback(async () => {
    setListLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<ApiKeySummary[]>('/rbac/keys');
    if (!result.ok) {
      setLoadFailed(true);
      setKeys([]);
    } else {
      setKeys(result.data);
    }
    setListLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return keys;
    return keys.filter((k) => k.name.toLowerCase().includes(q) || k.role.toLowerCase().includes(q));
  }, [keys, search]);

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
      void load();
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
      void load();
    } else {
      toast(response.error ?? 'Failed to revoke API key', 'error');
    }
  }

  if (listLoading && keys.length === 0 && !loadFailed) {
    return <PageLoading rows={5} />;
  }

  if (loadFailed) {
    return <PageLoadError title="RBAC keys unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search keys…"
        onRefresh={() => void load()}
        refreshing={listLoading}
      />

      <div className="dash-card mb-6">
        <div className="flex items-center gap-3 mb-4">
          <Shield className="w-5 h-5 text-aether" />
          <h2 className="text-lg font-semibold text-slate-100">Create RBAC API key</h2>
        </div>
        {canAdmin ? (
        <div className="grid grid-cols-1 md:grid-cols-[1.4fr_0.8fr_auto] gap-3" data-testid="rbac-create-form">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Key name, e.g. ci-bot"
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
            type="button"
            onClick={() => void handleCreate()}
            disabled={loading || !name.trim()}
            className="rounded-xl bg-aether hover:bg-aether/90 disabled:opacity-50 px-4 py-2.5 text-sm font-medium text-white"
          >
            {loading ? 'Creating…' : 'Create key'}
          </button>
        </div>
        ) : (
          <p className="text-sm text-slate-400">Only admin users can create or revoke API keys.</p>
        )}
        <Link to={viewToPath('audit')} className="mt-3 inline-flex text-xs text-aether hover:underline">
          View audit trail →
        </Link>
        <Link to={viewToPath('secrets')} className="mt-3 ml-4 inline-flex text-xs text-aether hover:underline" data-testid="rbac-secrets-link">
          Secrets vault →
        </Link>
      </div>

      {keys.length === 0 && !listLoading ? (
        <EmptyState icon={<KeyRound size={48} />} title="No RBAC keys" description="Create admin, operator, or viewer API keys." />
      ) : (
        <div className="dash-card overflow-hidden" data-testid="rbac-keys-list">
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
                {filtered.map((entry) => (
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
                      {canAdmin ? (
                      <button
                        type="button"
                        onClick={() => setRevokeName(entry.name)}
                        className="inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm text-red-300 hover:bg-red-500/10"
                      >
                        <Trash2 size={14} />
                        Revoke
                      </button>
                      ) : (
                        <span className="text-xs text-slate-500">—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {filtered.length === 0 && keys.length > 0 && (
              <p className="text-sm text-slate-500 py-6 text-center">No keys match your search.</p>
            )}
          </div>
        </div>
      )}

      <Modal isOpen={created !== null} onClose={() => setCreated(null)} title={`New API key: ${created?.name ?? ''}`}>
        {created && (
          <div className="space-y-4" data-testid="rbac-created-key">
            <p className="text-sm text-slate-400">This plaintext key is only returned once. Store it before closing.</p>
            <div className="rounded-xl border border-aether/20 bg-slate-950/80 px-4 py-3 font-mono text-sm text-aether break-all">
              {created.key}
            </div>
            <button
              type="button"
              data-testid="rbac-copy-key"
              onClick={() => {
                void navigator.clipboard.writeText(created.key);
                setKeyCopied(true);
                setTimeout(() => setKeyCopied(false), 2000);
              }}
              className="inline-flex items-center gap-2 rounded-lg border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:border-aether/40"
            >
              <Copy size={14} />
              {keyCopied ? 'Copied' : 'Copy key'}
            </button>
            <Badge text={created.role} variant={created.role === 'admin' ? 'red' : created.role === 'operator' ? 'yellow' : 'blue'} />
          </div>
        )}
      </Modal>

      <Modal isOpen={revokeName !== null} onClose={() => setRevokeName(null)} title="Revoke API key">
        <div data-testid="rbac-revoke-modal" className="space-y-4">
          <p className="text-sm text-slate-400">
            Revoke access for <span className="text-slate-200 font-medium">{revokeName}</span>?
          </p>
          <div className="flex justify-end gap-3">
            <button type="button" onClick={() => setRevokeName(null)} className="rounded-lg px-4 py-2 text-sm text-slate-300 hover:bg-slate-800">
              Cancel
            </button>
            <button type="button" onClick={() => void handleRevoke()} className="rounded-lg bg-red-600 hover:bg-red-500 px-4 py-2 text-sm font-medium text-white">
              Revoke
            </button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
