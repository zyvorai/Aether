import { useCallback, useEffect, useState } from 'react';
import { GitBranch } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
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

export default function GitOpsPage() {
  const [data, setData] = useState<GitOpsPayload | null>(null);
  const [syncResult, setSyncResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncConfirmOpen, setSyncConfirmOpen] = useState(false);

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

  const sync = async () => {
    setSyncing(true);
    setSyncResult(null);
    const res = await apiPost<{ changes?: GitOpsChangeRow[]; status?: unknown }>('/gitops/sync', {});
    setSyncing(false);
    if (res.success) {
      setSyncResult(JSON.stringify(res.data, null, 2));
      void load();
    } else {
      setSyncResult(res.error ?? 'Sync failed');
    }
  };

  const parsedSync = formatSyncResult(syncResult);

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
            onClick={() => setSyncConfirmOpen(true)}
            disabled={syncing || data?.configured === false}
            className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm font-medium text-aether hover:bg-aether/20 disabled:opacity-40"
          >
            {syncing ? 'Syncing…' : 'Sync now'}
          </button>
        }
      />

      <div className="dash-card">
        <div className="flex items-center gap-3 mb-4">
          <GitBranch className="w-5 h-5 text-aether" />
          <h2 className="text-lg font-semibold text-slate-100">GitOps reconciliation</h2>
          {data?.configured !== false && <Badge text="CONFIGURED" variant="green" />}
          {data?.configured === false && <Badge text="NOT CONFIGURED" variant="muted" />}
        </div>
        {loading ? (
          <p className="text-sm text-slate-500">Loading gitops status…</p>
        ) : data?.configured === false ? (
          <p className="text-sm text-slate-400 leading-relaxed">
            {data.hint ?? 'Not configured.'} On the server run:{' '}
            <code className="text-aether/90">aether git-ops init --repo &lt;URL&gt;</code>
          </p>
        ) : (
          <dl className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
            <div>
              <dt className="text-xs uppercase tracking-wider text-slate-500 mb-1">Repository</dt>
              <dd className="text-slate-200 break-all">{data?.repo_url ?? '—'}</dd>
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
      </div>

      {syncResult && (
        <div className="dash-card">
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
                      <td className="py-2 pr-4 font-mono text-xs text-slate-300">{c.file_path}</td>
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
              <h4 className="text-xs font-semibold uppercase tracking-wider text-slate-500 mb-2">
                Confidential compliance
              </h4>
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
                        <td className="py-2 pr-4 text-slate-200">{row.workload ?? '—'}</td>
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
        <p className="text-sm text-slate-300 mb-4">
          Pull from <span className="font-mono text-aether">{data?.repo_url ?? 'repository'}</span> and apply
          detected YAML changes. Review the diff preview after sync completes.
        </p>
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
            className="px-4 py-2 rounded-lg text-sm font-medium bg-aether text-white hover:bg-aether/90"
          >
            Sync now
          </button>
        </div>
      </Modal>
    </div>
  );
}
