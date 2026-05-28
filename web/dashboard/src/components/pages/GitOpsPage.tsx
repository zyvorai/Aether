// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { GitBranch, ExternalLink } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { viewToPath } from '../../utils/dashboardRoutes';
import { workloadNameFromGitOpsPath } from '../../utils/gitopsLinks';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Modal from '../Modal';
import Badge from '../Badge';
import type { GitOpsConfidentialAudit } from '../../types/api';

interface GitOpsPayload {
  configured?: boolean;
  hint?: string;
  repo_url?: string;
  branch?: string;
  last_sync?: string;
  status?: string | Record<string, unknown>;
  last_changes?: GitOpsChangeRow[];
  last_confidential_compliance?: GitOpsConfidentialAudit[];
}

function syncSnapshotFromStatus(data: GitOpsPayload | null): string | null {
  if (!data) return null;
  const changes = data.last_changes ?? [];
  const confidential = data.last_confidential_compliance ?? [];
  if (changes.length === 0 && confidential.length === 0) return null;
  return JSON.stringify(
    {
      message: data.last_sync ? `Last sync ${data.last_sync}` : 'Last sync',
      changes,
      confidential_compliance: confidential,
      status: data,
    },
    null,
    2,
  );
}

interface GitOpsChangeRow {
  file_path: string;
  change_type: string;
  commit: string;
}

function formatSyncResult(raw: string | null): {
  summary: string;
  details: Record<string, unknown> | null;
  changes: GitOpsChangeRow[];
  confidentialCompliance: GitOpsConfidentialAudit[];
} {
  if (!raw) return { summary: '', details: null, changes: [], confidentialCompliance: [] };
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const msg =
      typeof parsed.message === 'string'
        ? parsed.message
        : typeof parsed.status === 'string'
          ? parsed.status
          : 'Sync completed';
    const changes = Array.isArray(parsed.changes)
      ? (parsed.changes as GitOpsChangeRow[]).filter((c) => c && typeof c.file_path === 'string')
      : [];
    const confidentialCompliance = Array.isArray(parsed.confidential_compliance)
      ? (parsed.confidential_compliance as GitOpsConfidentialAudit[]).filter(
          (row) => row && typeof row.file_path === 'string',
        )
      : [];
    return { summary: msg, details: parsed, changes, confidentialCompliance };
  } catch {
    return { summary: raw, details: null, changes: [], confidentialCompliance: [] };
  }
}

const GITOPS_SYNC_STORAGE_KEY = 'aether-gitops-last-sync';

export default function GitOpsPage() {
  const navigate = useNavigate();
  const [workloadQuery] = useQueryParam('workload', '');
  const workloadFocus = workloadQuery.trim();
  const [data, setData] = useState<GitOpsPayload | null>(null);
  const [syncResult, setSyncResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncConfirmOpen, setSyncConfirmOpen] = useState(false);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewChanges, setPreviewChanges] = useState<GitOpsChangeRow[]>([]);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [initRepo, setInitRepo] = useState('');
  const [initBranch, setInitBranch] = useState('main');
  const [initializing, setInitializing] = useState(false);
  const [initMessage, setInitMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<GitOpsPayload>('/gitops/status');
    if (!result.ok) {
      setLoadFailed(true);
      setData(null);
    } else {
      setData(result.data ?? { configured: false });
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const fromApi = syncSnapshotFromStatus(data);
    if (fromApi) {
      setSyncResult(fromApi);
      try {
        sessionStorage.setItem(GITOPS_SYNC_STORAGE_KEY, fromApi);
      } catch {
        /* ignore storage errors */
      }
      return;
    }
    try {
      const stored = sessionStorage.getItem(GITOPS_SYNC_STORAGE_KEY);
      if (stored) setSyncResult(stored);
    } catch {
      /* ignore storage errors */
    }
  }, [data]);

  const persistSyncResult = (raw: string) => {
    setSyncResult(raw);
    try {
      sessionStorage.setItem(GITOPS_SYNC_STORAGE_KEY, raw);
    } catch {
      /* ignore storage errors */
    }
  };

  const initGitOps = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!initRepo.trim()) return;
    setInitializing(true);
    setInitMessage(null);
    const res = await apiPost<{ repo_url?: string; repo_dir?: string }>('/gitops/init', {
      repo: initRepo.trim(),
      branch: initBranch.trim() || 'main',
    });
    setInitializing(false);
    if (res.success) {
      setInitMessage(`Initialized ${res.data?.repo_url ?? initRepo}`);
      void load();
    } else {
      setInitMessage(res.error ?? 'Init failed');
    }
  };

  const sync = async () => {
    setSyncing(true);
    setSyncResult(null);
    const res = await apiPost<{ changes?: GitOpsChangeRow[]; status?: unknown }>('/gitops/sync', {});
    setSyncing(false);
    if (res.success) {
      persistSyncResult(JSON.stringify(res.data, null, 2));
      void load();
    } else {
      setSyncResult(res.error ?? 'Sync failed');
    }
  };

  const openSyncConfirm = async () => {
    setSyncConfirmOpen(true);
    setPreviewLoading(true);
    setPreviewError(null);
    setPreviewChanges([]);
    const res = await apiPost<{ changes?: GitOpsChangeRow[] }>('/gitops/preview', {});
    setPreviewLoading(false);
    if (res.success && res.data?.changes) {
      setPreviewChanges(res.data.changes);
    } else if (!res.success) {
      setPreviewError(res.error ?? 'Could not load diff preview');
    }
  };

  const parsedSync = formatSyncResult(syncResult);

  function openWorkloadFromPath(filePath: string) {
    const name = workloadNameFromGitOpsPath(filePath);
    if (!name) return;
    navigate(pathWithQuery(viewToPath('workloads'), { workload: name }));
  }

  function GitOpsFileCell({ filePath }: { filePath: string }) {
    const workloadName = workloadNameFromGitOpsPath(filePath);
    if (!workloadName) {
      return <span className="font-mono text-xs text-zinc-300">{filePath}</span>;
    }
    return (
      <button
        type="button"
        onClick={() => openWorkloadFromPath(filePath)}
        className="font-mono text-xs text-aether hover:underline text-left"
        title={`Open workload ${workloadName}`}
      >
        {filePath}
      </button>
    );
  }

  if (loading && !data && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="GitOps status unavailable" onRetry={() => void load()} />;
  }

  return (
    <div className="space-y-6">
      <PageToolbar
        onRefresh={() => void load()}
        refreshing={loading || syncing}
        actions={
          <button
            type="button"
            data-testid="gitops-sync-now"
            onClick={() => void openSyncConfirm()}
            disabled={syncing || data?.configured === false}
            className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm font-medium text-aether hover:bg-aether/20 disabled:opacity-40"
          >
            {syncing ? 'Syncing…' : 'Sync now'}
          </button>
        }
      />

      {workloadFocus ? (
        <WorkloadContextBanner
          testId="gitops-workload-context"
          workload={workloadFocus}
          description="GitOps context for workload"
        >
          <WorkloadScopedCrossLinks workload={workloadFocus} prefix="gitops" showDrift showAudit showMetrics />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="gitops-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="gitops-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="gitops-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="gitops-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="gitops-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="gitops-context-compose-link"
          >
            Compose →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <div className="dash-card" data-testid="gitops-status-panel">
        <div className="flex items-center gap-3 mb-4">
          <GitBranch className="w-5 h-5 text-aether" />
          <h2 className="text-lg font-semibold text-slate-100">GitOps reconciliation</h2>
          {data?.configured !== false && <Badge text="CONFIGURED" variant="green" />}
          {data?.configured === false && <Badge text="NOT CONFIGURED" variant="muted" />}
        </div>
        {loading ? (
          <p className="text-sm text-slate-500">Loading gitops status…</p>
        ) : data?.configured === false ? (
          <div className="space-y-4">
            <p className="text-sm text-slate-400 leading-relaxed">
              {data.hint ?? 'GitOps is not configured on this server.'}
            </p>
            <form onSubmit={(e) => void initGitOps(e)} className="grid grid-cols-1 sm:grid-cols-2 gap-3 max-w-2xl">
              <div className="sm:col-span-2">
                <label className="block text-xs text-slate-500 mb-1">Repository URL</label>
                <input
                  type="url"
                  value={initRepo}
                  onChange={(e) => setInitRepo(e.target.value)}
                  placeholder="https://github.com/org/aether-workloads.git"
                  className="w-full rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm text-slate-100"
                />
              </div>
              <div>
                <label className="block text-xs text-slate-500 mb-1">Branch</label>
                <input
                  type="text"
                  value={initBranch}
                  onChange={(e) => setInitBranch(e.target.value)}
                  className="w-full rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm text-slate-100"
                />
              </div>
              <div className="flex items-end">
                <button
                  type="submit"
                  disabled={initializing || !initRepo.trim()}
                  className="rounded-xl bg-aether/20 border border-aether/40 px-4 py-2 text-sm text-aether hover:bg-aether/30 disabled:opacity-50"
                >
                  {initializing ? 'Initializing…' : 'Initialize GitOps'}
                </button>
              </div>
            </form>
            {initMessage && <p className="text-sm text-slate-400">{initMessage}</p>}
          </div>
        ) : (
          <dl className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
            <div>
              <dt className="text-xs uppercase tracking-wider text-slate-500 mb-1">Repository</dt>
              <dd className="text-slate-200 break-all">
                {data?.repo_url && /^https?:\/\//i.test(data.repo_url) ? (
                  <a
                    href={data.repo_url}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex items-center gap-1.5 text-aether hover:underline"
                    data-testid="gitops-repo-external-link"
                  >
                    {data.repo_url}
                    <ExternalLink size={14} />
                  </a>
                ) : (
                  data?.repo_url ?? '—'
                )}
              </dd>
            </div>
            <div>
              <dt className="text-xs uppercase tracking-wider text-slate-500 mb-1">Branch</dt>
              <dd className="text-slate-200">{data?.branch ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-xs uppercase tracking-wider text-slate-500 mb-1">Last sync</dt>
              <dd className="text-slate-200">
                {data?.last_sync ? formatTimestamp(data.last_sync) : '—'}
              </dd>
            </div>
            <div>
              <dt className="text-xs uppercase tracking-wider text-slate-500 mb-1">Status</dt>
              <dd className="text-slate-200">
                {typeof data?.status === 'string'
                  ? data.status
                  : data?.status
                    ? JSON.stringify(data.status)
                    : '—'}
              </dd>
            </div>
          </dl>
        )}
        <button
          type="button"
          data-testid="gitops-drift-link"
          onClick={() =>
            navigate(
              workloadFocus
                ? pathWithQuery(viewToPath('drift'), { workload: workloadFocus })
                : viewToPath('drift'),
            )
          }
          className="mt-4 mr-4 text-xs text-aether hover:underline"
        >
          Drift detection →
        </button>
        <button
          type="button"
          data-testid="gitops-policy-link"
          onClick={() => navigate(viewToPath('policy'))}
          className="mt-4 text-xs text-aether hover:underline"
        >
          Policy check →
        </button>
      </div>

      {syncResult && (
        <div className="dash-card" data-testid="gitops-sync-result">
          <h3 className="text-sm font-semibold text-slate-100 mb-3">Last sync result</h3>
          <p className="text-sm text-slate-300 mb-3">{parsedSync.summary}</p>
          {parsedSync.changes.length > 0 && (
            <div className="mb-4 overflow-x-auto">
              <table className="w-full text-sm border-collapse">
                <thead>
                  <tr className="border-b border-slate-800 text-left text-xs uppercase tracking-wider text-slate-500">
                    <th className="py-2 pr-4">Change</th>
                    <th className="py-2 pr-4">File</th>
                    <th className="py-2 pr-4">Confidential</th>
                    <th className="py-2">Commit</th>
                  </tr>
                </thead>
                <tbody>
                  {parsedSync.changes.map((c) => {
                    const audit = parsedSync.confidentialCompliance.find((row) => row.file_path === c.file_path);
                    const confidentialLabel = !audit
                      ? '—'
                      : !audit.confidential_enabled
                        ? 'off'
                        : audit.gitops_issues.length > 0 || audit.sovereign_compliant === false
                          ? 'issues'
                          : 'ok';
                    const confidentialVariant =
                      confidentialLabel === 'ok'
                        ? 'green'
                        : confidentialLabel === 'issues'
                          ? 'red'
                          : confidentialLabel === 'off'
                            ? 'muted'
                            : 'muted';
                    return (
                    <tr key={`${c.commit}-${c.file_path}`} className="border-b border-slate-800/50">
                      <td className="py-2 pr-4">
                        <Badge
                          text={c.change_type}
                          variant={
                            c.change_type === 'Added'
                              ? 'green'
                              : c.change_type === 'Deleted'
                                ? 'red'
                                : 'yellow'
                          }
                        />
                      </td>
                      <td className="py-2 pr-4 font-mono text-xs text-zinc-300">
                        <GitOpsFileCell filePath={c.file_path} />
                      </td>
                      <td className="py-2 pr-4">
                        <Badge text={confidentialLabel} variant={confidentialVariant} />
                      </td>
                      <td className="py-2 font-mono text-xs text-slate-500 truncate max-w-[12rem]" title={c.commit}>
                        {c.commit.slice(0, 12)}
                      </td>
                    </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
          {parsedSync.confidentialCompliance.some((row) => row.confidential_enabled) && (
            <div className="mb-4 overflow-x-auto">
              <div className="flex flex-wrap items-center justify-between gap-3 mb-2">
                <h4 className="text-xs font-semibold uppercase tracking-wider text-slate-500">
                  Confidential compliance
                </h4>
                <button
                  type="button"
                  data-testid="gitops-confidential-link"
                  onClick={() => navigate(viewToPath('confidential'))}
                  className="text-xs text-aether hover:underline"
                >
                  Open confidential page →
                </button>
              </div>
              <table className="w-full text-sm border-collapse">
                <thead>
                  <tr className="border-b border-slate-800 text-left text-xs uppercase tracking-wider text-slate-500">
                    <th className="py-2 pr-4">Workload</th>
                    <th className="py-2 pr-4">File</th>
                    <th className="py-2 pr-4">GitOps issues</th>
                    <th className="py-2">Sovereign</th>
                  </tr>
                </thead>
                <tbody>
                  {parsedSync.confidentialCompliance
                    .filter((row) => row.confidential_enabled)
                    .map((row) => (
                      <tr key={row.file_path} className="border-b border-slate-800/50 align-top">
                        <td className="py-2 pr-4 text-slate-200">
                          {row.workload ? (
                            <button
                              type="button"
                              onClick={() =>
                                navigate(
                                  pathWithQuery(viewToPath('workloads'), {
                                    workload: row.workload!,
                                    tab: 'trust',
                                  }),
                                )
                              }
                              className="text-aether hover:underline"
                              data-testid={`gitops-confidential-row-${row.workload}`}
                            >
                              {row.workload}
                            </button>
                          ) : (
                            '—'
                          )}
                        </td>
                        <td className="py-2 pr-4 font-mono text-xs text-slate-400">{row.file_path}</td>
                        <td className="py-2 pr-4 text-xs text-amber-200/90">
                          {row.gitops_issues.length > 0 ? row.gitops_issues.join('; ') : '—'}
                        </td>
                        <td className="py-2">
                          <Badge
                            text={row.sovereign_compliant ? 'compliant' : 'violations'}
                            variant={row.sovereign_compliant ? 'green' : 'red'}
                          />
                          {row.sovereign_violations.length > 0 && (
                            <p className="mt-1 text-xs text-red-300/90">{row.sovereign_violations.join('; ')}</p>
                          )}
                        </td>
                      </tr>
                    ))}
                </tbody>
              </table>
            </div>
          )}
          {parsedSync.details && (
            <dl className="space-y-2 text-sm">
              {Object.entries(parsedSync.details)
                .filter(([key]) => key !== 'changes' && key !== 'confidential_compliance')
                .map(([key, value]) => (
                  <div key={key} className="flex gap-2">
                    <dt className="text-slate-500 shrink-0">{key}:</dt>
                    <dd className="text-slate-300 break-all">
                      {typeof value === 'object' ? JSON.stringify(value) : String(value)}
                    </dd>
                  </div>
                ))}
            </dl>
          )}
        </div>
      )}

      <Modal isOpen={syncConfirmOpen} onClose={() => setSyncConfirmOpen(false)} title="Confirm GitOps sync">
        <div data-testid="gitops-sync-confirm">
        <p className="text-sm text-slate-300 mb-4">
          Pull from <span className="font-mono text-aether">{data?.repo_url ?? 'repository'}</span> and apply
          detected YAML changes. Review the diff preview below before syncing.
        </p>
        <div
          className="mb-4 rounded-xl border border-zinc-800 bg-zinc-950/60 p-3"
          data-testid="gitops-diff-preview"
        >
          {previewLoading ? (
            <p className="text-sm text-zinc-500">Loading pending changes…</p>
          ) : previewError ? (
            <p className="text-sm text-red-400">{previewError}</p>
          ) : previewChanges.length === 0 ? (
            <p className="text-sm text-zinc-500">No YAML changes detected in the latest commit.</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm border-collapse">
                <thead>
                  <tr className="border-b border-zinc-800 text-left text-xs uppercase tracking-wider text-zinc-500">
                    <th className="py-2 pr-4">Change</th>
                    <th className="py-2 pr-4">File</th>
                    <th className="py-2">Commit</th>
                  </tr>
                </thead>
                <tbody>
                  {previewChanges.map((c) => (
                    <tr key={`${c.commit}-${c.file_path}`} className="border-b border-zinc-800/50">
                      <td className="py-2 pr-4">
                        <Badge
                          text={c.change_type}
                          variant={
                            c.change_type === 'Added'
                              ? 'green'
                              : c.change_type === 'Deleted'
                                ? 'red'
                                : 'yellow'
                          }
                        />
                      </td>
                      <td className="py-2 pr-4 font-mono text-xs text-zinc-300">
                        <GitOpsFileCell filePath={c.file_path} />
                      </td>
                      <td className="py-2 font-mono text-xs text-zinc-500 truncate max-w-[12rem]" title={c.commit}>
                        {c.commit.slice(0, 12)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setSyncConfirmOpen(false)}
            className="px-4 py-2 rounded-lg text-sm bg-zinc-800 text-zinc-200 hover:bg-zinc-700"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              setSyncConfirmOpen(false);
              void sync();
            }}
            data-testid="gitops-sync-confirm-button"
            className="px-4 py-2 rounded-lg text-sm font-medium bg-aether text-white hover:bg-aether/90"
          >
            Sync now
          </button>
        </div>
        </div>
      </Modal>
    </div>
  );
}
