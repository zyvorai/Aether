import { useCallback, useEffect, useState } from 'react';
import { GitBranch, RefreshCw } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';

interface GitOpsPayload {
  configured?: boolean;
  hint?: string;
  repo_url?: string;
  branch?: string;
  last_sync?: string;
  status?: unknown;
}

export default function GitOpsPage() {
  const [data, setData] = useState<GitOpsPayload | null>(null);
  const [syncResult, setSyncResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const raw = await apiFetch<GitOpsPayload>('/gitops/status');
    setData(raw ?? { configured: false });
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const sync = async () => {
    setSyncing(true);
    setSyncResult(null);
    const res = await apiPost<{ changes?: unknown; status?: unknown }>('/gitops/sync', {});
    setSyncing(false);
    if (res.success) {
      setSyncResult(JSON.stringify(res.data, null, 2));
      void load();
    } else {
      setSyncResult(res.error ?? 'Sync failed');
    }
  };

  return (
    <section className="space-y-6">
      <div className="dash-card p-6">
        <div className="flex flex-wrap items-center justify-between gap-4 mb-4">
          <div className="flex items-center gap-3">
            <GitBranch className="w-5 h-5 text-aether" />
            <h2 className="text-lg font-semibold text-white">GitOps reconciliation</h2>
          </div>
          <button
            type="button"
            onClick={() => void sync()}
            disabled={syncing || data?.configured === false}
            className="inline-flex items-center gap-2 rounded-xl border border-aether/40 bg-aether/10 px-4 py-2 text-sm font-medium text-aether hover:bg-aether/20 disabled:opacity-40"
          >
            <RefreshCw className={`w-4 h-4 ${syncing ? 'animate-spin' : ''}`} />
            Sync now
          </button>
        </div>
        {loading ? (
          <p className="text-sm text-slate-500">Loading gitops.json…</p>
        ) : data?.configured === false ? (
          <p className="text-sm text-slate-400 leading-relaxed">
            {data.hint ?? 'Not configured.'} On the server run:{' '}
            <code className="text-aether/90">aether git-ops init --repo &lt;URL&gt;</code>
          </p>
        ) : (
          <pre className="max-h-64 overflow-auto rounded-xl border border-slate-700/50 bg-slate-950/80 p-4 text-xs text-slate-300">
            {JSON.stringify(data, null, 2)}
          </pre>
        )}
      </div>
      {syncResult && (
        <div className="dash-card p-6">
          <h3 className="text-sm font-semibold text-white mb-2">Last sync result</h3>
          <pre className="max-h-96 overflow-auto text-xs text-slate-300 whitespace-pre-wrap">{syncResult}</pre>
        </div>
      )}
    </section>
  );
}