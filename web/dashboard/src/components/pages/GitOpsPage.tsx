import { useCallback, useEffect, useState } from 'react';
import { GitBranch } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import Badge from '../Badge';

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
} {
  if (!raw) return { summary: '', details: null, changes: [] };
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
    return { summary: msg, details: parsed, changes };
  } catch {
    return { summary: raw, details: null, changes: [] };
  }
}

export default function GitOpsPage() {
  const [data, setData] = useState<GitOpsPayload | null>(null);
  const [syncResult, setSyncResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [syncing, setSyncing] = useState(false);

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
            onClick={() => void sync()}
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
                    <th className="py-2">Commit</th>
                  </tr>
                </thead>
                <tbody>
                  {parsedSync.changes.map((c) => (
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
                      <td className="py-2 font-mono text-xs text-slate-500 truncate max-w-[12rem]" title={c.commit}>
                        {c.commit.slice(0, 12)}
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
                .filter(([key]) => key !== 'changes')
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
    </div>
  );
}
