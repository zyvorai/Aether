// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
import SectionHubPage from '../SectionHubPage';
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
import GitOpsCenter from '../GitOpsCenter';
import GitOpsAgentPanel from '../GitOpsAgentPanel';
import IntentGitOpsDiffPanel from '../IntentGitOpsDiffPanel';
import DataTable, { type DataTableColumn } from '../ui/DataTable';
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

export type GitOpsSection = 'center' | 'agent' | 'intent' | 'sync';

export function GitOpsStudio({ refreshKey, forcedSection }: { refreshKey?: number; forcedSection?: GitOpsSection } = {}) {
  const show = (s: GitOpsSection) => !forcedSection || forcedSection === s;
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
  }, [refreshKey]);

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

  const changeBadgeVariant = (changeType: string) =>
    changeType === 'Added' ? 'green' : changeType === 'Deleted' ? 'red' : 'yellow';

  const syncChangeColumns: DataTableColumn<GitOpsChangeRow>[] = [
    {
      key: 'change',
      header: 'Change',
      width: 90,
      render: (c) => <Badge text={c.change_type} variant={changeBadgeVariant(c.change_type)} />,
    },
    {
      key: 'file',
      header: 'File',
      render: (c) => <GitOpsFileCell filePath={c.file_path} />,
    },
    {
      key: 'confidential',
      header: 'Confidential',
      width: 110,
      render: (c) => {
        const audit = parsedSync.confidentialCompliance.find((row) => row.file_path === c.file_path);
        const label = !audit
          ? '—'
          : !audit.confidential_enabled
            ? 'off'
            : audit.gitops_issues.length > 0 || audit.sovereign_compliant === false
              ? 'issues'
              : 'ok';
        const variant = label === 'ok' ? 'green' : label === 'issues' ? 'red' : 'muted';
        return <Badge text={label} variant={variant} />;
      },
    },
    {
      key: 'commit',
      header: 'Commit',
      width: 110,
      render: (c) => (
        <span className="font-mono text-xs text-subtle truncate" title={c.commit}>
          {c.commit.slice(0, 12)}
        </span>
      ),
    },
  ];

  const previewChangeColumns: DataTableColumn<GitOpsChangeRow>[] = [
    {
      key: 'change',
      header: 'Change',
      width: 90,
      render: (c) => <Badge text={c.change_type} variant={changeBadgeVariant(c.change_type)} />,
    },
    {
      key: 'file',
      header: 'File',
      render: (c) => <GitOpsFileCell filePath={c.file_path} />,
    },
    {
      key: 'commit',
      header: 'Commit',
      width: 110,
      render: (c) => (
        <span className="font-mono text-xs text-subtle truncate" title={c.commit}>
          {c.commit.slice(0, 12)}
        </span>
      ),
    },
  ];

  const confidentialColumns: DataTableColumn<GitOpsConfidentialAudit>[] = [
    {
      key: 'workload',
      header: 'Workload',
      render: (row) =>
        row.workload ? (
          <button
            type="button"
            onClick={() =>
              navigate(pathWithQuery(viewToPath('workloads'), { workload: row.workload!, tab: 'trust' }))
            }
            className="text-primary hover:underline"
            data-testid={`gitops-confidential-row-${row.workload}`}
          >
            {row.workload}
          </button>
        ) : (
          '—'
        ),
    },
    {
      key: 'file',
      header: 'File',
      render: (row) => <span className="font-mono text-xs text-muted">{row.file_path}</span>,
    },
    {
      key: 'issues',
      header: 'GitOps issues',
      render: (row) => (
        <span className="text-xs text-warning/90">
          {row.gitops_issues.length > 0 ? row.gitops_issues.join('; ') : '—'}
        </span>
      ),
    },
    {
      key: 'sovereign',
      header: 'Sovereign',
      width: 140,
      render: (row) => (
        <div>
          <Badge text={row.sovereign_compliant ? 'compliant' : 'violations'} variant={row.sovereign_compliant ? 'green' : 'red'} />
          {row.sovereign_violations.length > 0 && (
            <p className="mt-1 text-xs text-danger/90">{row.sovereign_violations.join('; ')}</p>
          )}
        </div>
      ),
    },
  ];

  function openWorkloadFromPath(filePath: string) {
    const name = workloadNameFromGitOpsPath(filePath);
    if (!name) return;
    navigate(pathWithQuery(viewToPath('workloads'), { workload: name }));
  }

  function GitOpsFileCell({ filePath }: { filePath: string }) {
    const workloadName = workloadNameFromGitOpsPath(filePath);
    if (!workloadName) {
      return <span className="font-mono text-xs text-muted">{filePath}</span>;
    }
    return (
      <button
        type="button"
        onClick={() => openWorkloadFromPath(filePath)}
        className="font-mono text-xs text-primary hover:underline text-left"
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
    <div className="space-y-12">
      <div className="mb-2 glass-context-banner" data-testid="gitops-hub-context">
        GitOps
        {' · '}
        <Link to={viewToPath('health')} className="text-primary hover:underline" data-testid="gitops-hub-orchestrator-link">
          Orchestrator →
        </Link>
        {' · '}
        <Link to={viewToPath('intelligence')} className="text-primary hover:underline" data-testid="gitops-hub-intelligence-link">
          Intelligence →
        </Link>
        {' · '}
        <Link to={`${viewToPath('fleet')}?tab=edge`} className="text-primary hover:underline" data-testid="gitops-hub-edge-link">
          Edge →
        </Link>
        {' · '}
        <Link to={viewToPath('hosted')} className="text-primary hover:underline" data-testid="gitops-hub-hosted-link">
          Hosted SaaS →
        </Link>
      </div>
      <PageToolbar
        onRefresh={() => void load()}
        refreshing={loading || syncing}
        actions={
          <button
            type="button"
            data-testid="gitops-sync-now"
            onClick={() => void openSyncConfirm()}
            disabled={syncing || data?.configured === false}
            className="inline-flex items-center gap-2 btn-primary disabled:opacity-40"
          >
            {syncing ? 'Syncing…' : 'Sync now'}
          </button>
        }
      />

      {show('center') ? (
      <GitOpsCenter
        configured={Boolean(data?.configured)}
        repoUrl={data?.repo_url}
        branch={data?.branch}
        lastSync={data?.last_sync}
        status={typeof data?.status === 'string' ? data.status : undefined}
        changes={parsedSync.changes}
        onSync={() => void openSyncConfirm()}
        onPreview={() => void openSyncConfirm()}
        syncing={syncing}
      />

      ) : null}

      {show('agent') ? <GitOpsAgentPanel /> : null}
      {show('intent') ? <IntentGitOpsDiffPanel /> : null}

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
            className="text-primary hover:underline"
            data-testid="gitops-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: workloadFocus })}
            className="text-primary hover:underline"
            data-testid="gitops-context-fleet-link"
          >
            Fleet →
          </Link>
          {' · '}
          <Link
            to={viewToPath('hosted')}
            className="text-primary hover:underline"
            data-testid="gitops-context-hosted-link"
          >
            Hosted SaaS →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: workloadFocus })}
            className="text-primary hover:underline"
            data-testid="gitops-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadFocus })}
            className="text-primary hover:underline"
            data-testid="gitops-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadFocus })}
            className="text-primary hover:underline"
            data-testid="gitops-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: workloadFocus })}
            className="text-primary hover:underline"
            data-testid="gitops-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: workloadFocus })}
            className="text-primary hover:underline"
            data-testid="gitops-context-compose-link"
          >
            Compose →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: workloadFocus })}
            className="text-primary hover:underline"
            data-testid="gitops-context-fleet-link"
          >
            Fleet →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      {show('sync') ? (
      <section className="apple-chapter space-y-8 mb-10">

      <div className="glass" data-testid="gitops-status-panel">
        <div className="flex items-center gap-3 mb-4">
          <GitBranch className="w-5 h-5 text-primary" />
          <h2 className="text-lg font-semibold text-foreground">GitOps reconciliation</h2>
          {data?.configured !== false && <Badge text="CONFIGURED" variant="green" />}
          {data?.configured === false && <Badge text="NOT CONFIGURED" variant="muted" />}
        </div>
        {loading ? (
          <p className="text-sm text-subtle">Loading gitops status…</p>
        ) : data?.configured === false ? (
          <div className="space-y-4">
            <p className="text-sm text-muted leading-relaxed">
              {data.hint ?? 'GitOps is not configured on this server.'}
            </p>
            <form onSubmit={(e) => void initGitOps(e)} className="grid grid-cols-1 sm:grid-cols-2 gap-3 max-w-2xl">
              <div className="sm:col-span-2">
                <label className="block text-xs text-subtle mb-1">Repository URL</label>
                <input
                  type="url"
                  value={initRepo}
                  onChange={(e) => setInitRepo(e.target.value)}
                  placeholder="https://github.com/org/aether-workloads.git"
                  className="glass-input text-foreground"
                />
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Branch</label>
                <input
                  type="text"
                  value={initBranch}
                  onChange={(e) => setInitBranch(e.target.value)}
                  className="glass-input text-foreground"
                />
              </div>
              <div className="flex items-end">
                <button
                  type="submit"
                  disabled={initializing || !initRepo.trim()}
                  className="btn-primary disabled:opacity-50"
                >
                  {initializing ? 'Initializing…' : 'Initialize GitOps'}
                </button>
              </div>
            </form>
            {initMessage && <p className="text-sm text-muted">{initMessage}</p>}
          </div>
        ) : (
          <dl className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
            <div>
              <dt className="text-xs uppercase tracking-wider text-subtle mb-1">Repository</dt>
              <dd className="text-foreground break-all">
                {data?.repo_url && /^https?:\/\//i.test(data.repo_url) ? (
                  <a
                    href={data.repo_url}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex items-center gap-1.5 text-primary hover:underline"
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
              <dt className="text-xs uppercase tracking-wider text-subtle mb-1">Branch</dt>
              <dd className="text-foreground">{data?.branch ?? '—'}</dd>
            </div>
            <div>
              <dt className="text-xs uppercase tracking-wider text-subtle mb-1">Last sync</dt>
              <dd className="text-foreground">
                {data?.last_sync ? formatTimestamp(data.last_sync) : '—'}
              </dd>
            </div>
            <div>
              <dt className="text-xs uppercase tracking-wider text-subtle mb-1">Status</dt>
              <dd className="text-foreground">
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
          className="mt-4 mr-4 text-xs text-primary hover:underline"
        >
          Drift detection →
        </button>
        <button
          type="button"
          data-testid="gitops-policy-link"
          onClick={() =>
            navigate(
              workloadFocus
                ? pathWithQuery(viewToPath('policy'), { workload: workloadFocus })
                : viewToPath('policy'),
            )
          }
          className="mt-4 text-xs text-primary hover:underline"
        >
          Policy check →
        </button>
      </div>

      {syncResult && (
        <div className="glass" data-testid="gitops-sync-result">
          <h3 className="text-sm font-semibold text-foreground mb-3">Last sync result</h3>
          <p className="text-sm text-muted mb-3">{parsedSync.summary}</p>
          {parsedSync.changes.length > 0 && (
            <div className="mb-4">
              <DataTable<GitOpsChangeRow>
                items={parsedSync.changes}
                getId={(c) => `${c.commit}-${c.file_path}`}
                columns={syncChangeColumns}
                sortBySeverityDefault={false}
              />
            </div>
          )}
          {parsedSync.confidentialCompliance.some((row) => row.confidential_enabled) && (
            <div className="mb-4">
              <div className="flex flex-wrap items-center justify-between gap-3 mb-2">
                <h4 className="text-xs font-semibold uppercase tracking-wider text-subtle">
                  Confidential compliance
                </h4>
                <button
                  type="button"
                  data-testid="gitops-confidential-link"
                  onClick={() => navigate(viewToPath('confidential'))}
                  className="text-xs text-primary hover:underline"
                >
                  Open confidential page →
                </button>
              </div>
              <DataTable<GitOpsConfidentialAudit>
                items={parsedSync.confidentialCompliance.filter((row) => row.confidential_enabled)}
                getId={(row) => row.file_path}
                columns={confidentialColumns}
                sortBySeverityDefault={false}
              />
            </div>
          )}
          {parsedSync.details && (
            <dl className="space-y-2 text-sm">
              {Object.entries(parsedSync.details)
                .filter(([key]) => key !== 'changes' && key !== 'confidential_compliance')
                .map(([key, value]) => (
                  <div key={key} className="flex gap-2">
                    <dt className="text-subtle shrink-0">{key}:</dt>
                    <dd className="text-muted break-all">
                      {typeof value === 'object' ? JSON.stringify(value) : String(value)}
                    </dd>
                  </div>
                ))}
            </dl>
          )}
        </div>
      )}
      </section>
      ) : null}


      <Modal isOpen={syncConfirmOpen} onClose={() => setSyncConfirmOpen(false)} title="Confirm GitOps sync">
        <div data-testid="gitops-sync-confirm">
        <p className="text-sm text-muted mb-4">
          Pull from <span className="font-mono text-primary">{data?.repo_url ?? 'repository'}</span> and apply
          detected YAML changes. Review the diff preview below before syncing.
        </p>
        <div
          className="mb-4 rounded-xl border glass-divider glass p-3"
          data-testid="gitops-diff-preview"
        >
          {previewLoading ? (
            <p className="text-sm text-subtle">Loading pending changes…</p>
          ) : previewError ? (
            <p className="text-sm text-danger">{previewError}</p>
          ) : previewChanges.length === 0 ? (
            <p className="text-sm text-subtle">No YAML changes detected in the latest commit.</p>
          ) : (
            <DataTable<GitOpsChangeRow>
              items={previewChanges}
              getId={(c) => `${c.commit}-${c.file_path}`}
              columns={previewChangeColumns}
              sortBySeverityDefault={false}
            />
          )}
        </div>
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setSyncConfirmOpen(false)}
            className="btn-secondary"
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
            className="btn-primary"
          >
            Sync now
          </button>
        </div>
        </div>
      </Modal>
    </div>
  );
}

function GitOpsHubPage() {
  return (
    <SectionHubPage
      links={[
        { view: 'gitops-center', title: 'GitOps center', description: 'Repo status and sync controls.', icon: <GitBranch className="h-5 w-5" /> },
        { view: 'gitops-agent', title: 'GitOps agent', description: 'Autonomous GitOps agent.', icon: <GitBranch className="h-5 w-5" /> },
        { view: 'gitops-intent', title: 'Intent diff', description: 'Intent vs GitOps drift.', icon: <GitBranch className="h-5 w-5" /> },
        { view: 'gitops-sync', title: 'Reconciliation', description: 'Init, sync, and change tables.', icon: <GitBranch className="h-5 w-5" /> },
      ]}
    />
  );
}

export default withAuroraPage('gitops', GitOpsHubPage);

