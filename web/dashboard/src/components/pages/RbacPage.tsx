// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState, useCallback, useMemo } from 'react';
import { Link } from 'react-router';
import { KeyRound, Shield, Trash2, Copy } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { copyToClipboard } from '../../utils/clipboard';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
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
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
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
        searchTestId="rbac-search"
        onRefresh={() => void load()}
        refreshing={listLoading}
      />

      {focusedWorkload ? (
        <WorkloadContextBanner testId="rbac-workload-context" workload={focusedWorkload} description="RBAC search context">
          <WorkloadScopedCrossLinks workload={focusedWorkload} prefix="rbac" showAudit />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('audit'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="rbac-audit-link"
          >
            Audit →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="rbac-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="rbac-context-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="rbac-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('gitops'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="rbac-context-gitops-link"
          >
            GitOps →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="rbac-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('confidential'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="rbac-context-confidential-link"
          >
            Confidential →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <section className="overview-section-shell mb-6 space-y-6 p-6 sm:p-8">

      <div className="glass-panel-card mb-6">
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
            className="glass-input"
          />
          <select
            value={role}
            onChange={(e) => setRole(e.target.value)}
            className="glass-input"
          >
            <option value="admin">Admin</option>
            <option value="operator">Operator</option>
            <option value="viewer">Viewer</option>
          </select>
          <button
            type="button"
            onClick={() => void handleCreate()}
            disabled={loading || !name.trim()}
            className="btn-primary disabled:opacity-50"
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
        <Link to={viewToPath('secrets')} className="mt-3 ml-4 inline-flex text-xs text-aether hover:underline" data-testid="rbac-vault-link">
          Secrets vault →
        </Link>
      </div>

      {keys.length === 0 && !listLoading ? (
        <EmptyState icon={<KeyRound size={48} />} title="No RBAC keys" description="Create admin, operator, or viewer API keys." />
      ) : filtered.length === 0 ? (
        <EmptyState icon={<KeyRound size={48} />} title="No matching keys" description="No keys match your search." />
      ) : (
        <CardGrid columns="compact" testId="rbac-keys-list">
          {filtered.map((entry, i) => (
            <EntityCard
              key={entry.name}
              index={i}
              testId={`rbac-key-card-${entry.name}`}
              icon={<KeyRound size={18} />}
              statusTone={entry.role === 'admin' ? 'red' : entry.role === 'operator' ? 'amber' : 'sky'}
              title={entry.name}
              subtitle={entry.created_at}
              badge={<Badge text={entry.role} variant={entry.role === 'admin' ? 'red' : entry.role === 'operator' ? 'yellow' : 'blue'} />}
              footer={
                canAdmin ? (
                  <button
                    type="button"
                    onClick={() => setRevokeName(entry.name)}
                    className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-red-500/15 hover:text-red-300"
                  >
                    <Trash2 size={13} />
                    Revoke
                  </button>
                ) : (
                  <span className="flex-1 py-1.5 text-center text-[11px] text-slate-500">Read-only</span>
                )
              }
            />
          ))}
        </CardGrid>
      )}
      </section>


      <Modal isOpen={created !== null} onClose={() => setCreated(null)} title={`New API key: ${created?.name ?? ''}`}>
        {created && (
          <div className="space-y-4" data-testid="rbac-created-key">
            <p className="text-sm text-slate-400">This plaintext key is only returned once. Store it before closing.</p>
            <div className="glass-code-block-body px-4 py-3 font-mono text-sm text-aether break-all">
              {created.key}
            </div>
            <button
              type="button"
              data-testid="rbac-copy-key"
              onClick={() => {
                void copyToClipboard(created.key);
                setKeyCopied(true);
                setTimeout(() => setKeyCopied(false), 2000);
              }}
              className="inline-flex items-center gap-2 rounded-lg border glass-divider px-3 py-1.5 text-xs text-slate-300 hover:border-aether/40"
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
            <button type="button" onClick={() => setRevokeName(null)} className="rounded-lg px-4 py-2 text-sm text-slate-300 glass-inset-hover">
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
