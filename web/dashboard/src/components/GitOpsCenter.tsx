// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { GitBranch, CheckCircle2, AlertTriangle, XCircle } from 'lucide-react';
import Badge from './Badge';

interface GitOpsChangeRow {
  file_path: string;
  change_type: string;
  commit: string;
}

interface GitOpsCenterProps {
  configured: boolean;
  repoUrl?: string | null;
  branch?: string | null;
  lastSync?: string | null;
  status?: string | null;
  changes: GitOpsChangeRow[];
  onSync: () => void;
  onPreview: () => void;
  syncing: boolean;
}

function humanChange(c: GitOpsChangeRow): string {
  const file = c.file_path.split('/').pop() ?? c.file_path;
  switch (c.change_type.toLowerCase()) {
    case 'added':
      return `Added ${file}`;
    case 'modified':
      return `Updated ${file}`;
    case 'deleted':
      return `Removed ${file}`;
    default:
      return `${c.change_type} ${file}`;
  }
}

export default function GitOpsCenter({
  configured,
  repoUrl,
  branch,
  lastSync,
  status,
  changes,
  onPreview,
  syncing: _syncing,
}: GitOpsCenterProps) {
  const synced = changes.length === 0 && configured;
  const outOfSync = changes.length > 0;
  const failed = status?.toLowerCase().includes('fail') ?? false;

  return (
    <section className="command-center-shell mb-6 p-6 sm:p-8" data-testid="gitops-center">
      <div className="flex flex-col lg:flex-row lg:items-start lg:justify-between gap-4 mb-6">
        <div>
          <p className="section-label">GitOps</p>
          <h2 className="section-title mt-1">Repository sync status</h2>
          {repoUrl && (
            <p className="text-xs text-slate-500 mt-1 font-mono truncate max-w-xl">
              {repoUrl} @ {branch ?? 'main'}
            </p>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={onPreview}
            className="btn-secondary"
          >
            View Diff
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 mb-6">
        <div className="glass-panel-card border-emerald-500/20 bg-emerald-950/20 py-4">
          <div className="flex items-center gap-2 text-emerald-300">
            <CheckCircle2 size={18} />
            <span className="text-sm font-medium">Synced</span>
          </div>
          <p className="text-2xl font-semibold text-white mt-2">{synced ? 'Yes' : outOfSync ? 'Partial' : '—'}</p>
        </div>
        <div className="glass-panel-card border-amber-500/20 bg-amber-950/20 py-4">
          <div className="flex items-center gap-2 text-amber-300">
            <AlertTriangle size={18} />
            <span className="text-sm font-medium">Out of sync</span>
          </div>
          <p className="text-2xl font-semibold text-white mt-2">{changes.length}</p>
        </div>
        <div className="glass-panel-card border-red-500/20 bg-red-950/20 py-4">
          <div className="flex items-center gap-2 text-red-300">
            <XCircle size={18} />
            <span className="text-sm font-medium">Failed</span>
          </div>
          <p className="text-2xl font-semibold text-white mt-2">{failed ? 1 : 0}</p>
        </div>
      </div>

      {!configured ? (
        <p className="text-sm text-slate-500">
          GitOps is not configured. Run <code className="text-slate-400">aether git-ops init</code> to connect a repository.
        </p>
      ) : changes.length === 0 ? (
        <div className="flex items-center gap-2 text-sm text-emerald-300">
          <GitBranch size={16} />
          All apps synced{lastSync ? ` · Last sync ${lastSync}` : ''}
        </div>
      ) : (
        <div className="space-y-2">
          <p className="text-xs uppercase tracking-wide text-slate-500 mb-2">Pending changes</p>
          {changes.slice(0, 8).map((c) => (
            <div
              key={`${c.file_path}-${c.commit}`}
              className="flex items-center justify-between gap-3 rounded-xl border border-slate-800/60 bg-[#11151C]/50 px-3 py-2 backdrop-blur-sm"
            >
              <span className="text-sm text-slate-200">{humanChange(c)}</span>
              <Badge text={c.change_type} variant="yellow" />
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
