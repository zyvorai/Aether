// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Plus, Inbox, RotateCcw } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import { Link, useNavigate } from 'react-router';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useWorkloadOrSearchFilter } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Modal from '../Modal';
import { SearchQueryContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import type { BackupInfo } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function BackupsPage() {
  const navigate = useNavigate();
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [search, setSearch] = useWorkloadOrSearchFilter();
  const [backupName, setBackupName] = useState('');
  const [backupDescription, setBackupDescription] = useState('');
  const [restoreOpen, setRestoreOpen] = useState<string | null>(null);
  const [restoreMerge, setRestoreMerge] = useState(false);
  const [restoring, setRestoring] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<BackupInfo[]>('/backups');
    if (!result.ok) {
      setLoadFailed(true);
      setBackups([]);
    } else {
      setBackups(result.data);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return backups;
    return backups.filter(
      (b) =>
        b.filename.toLowerCase().includes(q) ||
        (b.description?.toLowerCase().includes(q) ?? false)
    );
  }, [backups, search]);

  async function handleRestore() {
    if (!restoreOpen) return;
    setRestoring(true);
    const stem = restoreOpen.replace(/\.json$/i, '');
    const res = await apiPost('/backups/restore', { name: stem, merge: restoreMerge });
    setRestoring(false);
    setRestoreOpen(null);
    setRestoreMerge(false);
    if (res.success) {
      toast(`Restored from backup "${stem}"`, 'success');
      void load();
    } else {
      toast(res.error ?? 'Restore failed', 'error');
    }
  }

  async function handleCreate() {
    setCreating(true);
    await apiPost('/backups', {
      name: backupName.trim() || undefined,
      description: backupDescription.trim() || undefined,
    });
    setCreating(false);
    setCreateOpen(false);
    setBackupName('');
    setBackupDescription('');
    toast('Backup created', 'success');
    void load();
  }

  if (loading && backups.length === 0 && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Backups unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <div className="mb-6 glass-context-banner" data-testid="backups-hub-context">
        Backups hub
        {' · '}
        <Link to={viewToPath('settings')} className="text-aether hover:underline" data-testid="backups-context-settings-link">
          Settings →
        </Link>
        {' · '}
        <Link to={viewToPath('audit')} className="text-aether hover:underline" data-testid="backups-context-audit-link">
          Audit →
        </Link>
        {' · '}
        <Link to={viewToPath('hosted')} className="text-aether hover:underline" data-testid="backups-context-hosted-link">
          Hosted SaaS →
        </Link>
        {' · '}
        <Link to={viewToPath('migrations')} className="text-aether hover:underline" data-testid="backups-context-migrations-link">
          Migrations →
        </Link>
      </div>
      <SearchQueryContextBanner testId="backups-workload-context" query={search} entityLabel="backups">
        <WorkloadScopedCrossLinks workload={search} prefix="backups" showGitops />
        {search.trim() ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="backups-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('drift'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="backups-drift-link"
            >
              Drift →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="backups-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('rbac'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="backups-context-rbac-link"
            >
              RBAC →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('gitops'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="backups-context-gitops-link"
            >
              GitOps →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: search.trim() })}
              className="text-aether hover:underline"
              data-testid="backups-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('copilot'), { workload: search.trim(), q: `Backup guidance for ${search.trim()}` })}
              className="text-aether hover:underline"
              data-testid="backups-context-copilot-link"
            >
              Copilot →
            </Link>
          </>
        ) : null}
      </SearchQueryContextBanner>

      <section className="overview-section-shell mb-6 p-6 sm:p-8">
      <div className="mb-4 flex flex-wrap gap-3">
        <button
          type="button"
          data-testid="backups-audit-link"
          onClick={() =>
            navigate(
              search.trim()
                ? pathWithQuery(viewToPath('audit'), { workload: search.trim() })
                : viewToPath('audit'),
            )
          }
          className="text-xs text-aether hover:underline"
        >
          View restore audit trail →
        </button>
        <button
          type="button"
          data-testid="backups-platform-link"
          onClick={() =>
            navigate(
              search.trim()
                ? pathWithQuery(viewToPath('platform'), { workload: search.trim() })
                : viewToPath('platform'),
            )
          }
          className="text-xs text-aether hover:underline"
        >
          Remote backup config →
        </button>
      </div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search backups…"
        onRefresh={() => void load()}
        refreshing={loading}
        actions={
          <button
            type="button"
            data-testid="backups-create-button"
            onClick={() => setCreateOpen(true)}
            className="inline-flex items-center gap-2 btn-primary"
          >
            <Plus size={16} />
            Create backup
          </button>
        }
      />

      {backups.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No backups" description="Create a backup to get started" />
      ) : (
        <div className="glass-panel-card overflow-hidden" data-testid="backups-list">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="glass-divider-b">
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">File</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Workloads</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Created</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Version</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((b) => (
                  <tr key={b.filename} className="glass-table-row glass-inset-hover transition-colors">
                    <td className="py-3 px-4">
                      <code className="glass-table-row text-xs px-2 py-1 rounded text-slate-300">{b.filename}</code>
                      {b.description && <p className="text-xs text-slate-500 mt-1">{b.description}</p>}
                    </td>
                    <td className="py-3 px-4 text-sm text-slate-300">{b.workload_count}</td>
                    <td className="py-3 px-4 text-sm text-slate-400">{formatTimestamp(b.created_at)}</td>
                    <td className="py-3 px-4 text-sm text-slate-400">{b.aether_version}</td>
                    <td className="py-3 px-4">
                      <button
                        type="button"
                        onClick={() => setRestoreOpen(b.filename)}
                        className="inline-flex items-center gap-1.5 rounded-lg border glass-divider px-2.5 py-1 text-xs text-slate-300 hover:border-emerald-600/50 hover:text-emerald-300"
                      >
                        <RotateCcw size={12} />
                        Restore
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {filtered.length === 0 && (
              <p className="text-sm text-slate-500 py-6 text-center">No backups match your search.</p>
            )}
          </div>
        </div>
      )}
      </section>


      <Modal isOpen={createOpen} onClose={() => setCreateOpen(false)} title="Create backup">
        <div data-testid="backup-create-modal" className="space-y-4">
          <input
            value={backupName}
            onChange={(e) => setBackupName(e.target.value)}
            placeholder="Optional backup name"
            className="glass-input"
          />
          <textarea
            value={backupDescription}
            onChange={(e) => setBackupDescription(e.target.value)}
            placeholder="Optional description"
            rows={4}
            className="glass-input"
          />
          <div className="flex justify-end gap-3">
            <button type="button" onClick={() => setCreateOpen(false)} className="px-4 py-2 rounded-lg text-sm text-slate-300 glass-inset-hover">
              Cancel
            </button>
            <button
              type="button"
              onClick={() => void handleCreate()}
              disabled={creating}
              className="btn-primary disabled:opacity-50"
            >
              {creating ? 'Creating…' : 'Create backup'}
            </button>
          </div>
        </div>
      </Modal>

      <Modal isOpen={restoreOpen !== null} onClose={() => setRestoreOpen(null)} title="Restore backup">
        <div data-testid="backup-restore-modal">
        <p className="text-sm text-slate-300 mb-4">
          Restore state from <code className="text-slate-400">{restoreOpen}</code>. This replaces current workloads unless merge is enabled.
        </p>
        <label className="flex items-center gap-2 text-sm text-slate-300 mb-6">
          <input
            type="checkbox"
            checked={restoreMerge}
            onChange={(e) => setRestoreMerge(e.target.checked)}
            className="rounded glass-divider"
          />
          Merge workloads not already present
        </label>
        <div className="flex justify-end gap-3">
          <button type="button" onClick={() => setRestoreOpen(null)} className="px-4 py-2 rounded-lg text-sm text-slate-300 glass-inset-hover">
            Cancel
          </button>
          <button
            type="button"
            onClick={() => void handleRestore()}
            disabled={restoring}
            className="btn-primary disabled:opacity-50"
          >
            {restoring ? 'Restoring…' : 'Restore'}
          </button>
        </div>
        </div>
      </Modal>
    </div>
  );
}
