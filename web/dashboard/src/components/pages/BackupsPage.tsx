import { useState, useEffect } from 'react';
import { Plus, Inbox } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import EmptyState from '../EmptyState';
import type { BackupInfo } from '../../types/api';

export default function BackupsPage() {
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);

  async function load() {
    setLoading(true);
    const data = await apiFetch<BackupInfo[]>('/backups');
    setBackups(data ?? []);
    setLoading(false);
  }

  useEffect(() => { load(); }, []);

  async function handleCreate() {
    setCreating(true);
    await apiPost('/backups');
    setCreating(false);
    load();
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-end mb-6">
        <button
          onClick={handleCreate}
          disabled={creating}
          className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
        >
          <Plus size={16} />
          {creating ? 'Creating...' : 'Create Backup'}
        </button>
      </div>

      {backups.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No backups" description="Create a backup to get started" />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">File</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Workloads</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Created</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Version</th>
                </tr>
              </thead>
              <tbody>
                {backups.map((b) => (
                  <tr key={b.filename} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-4">
                      <code className="text-xs bg-zinc-950 px-2 py-1 rounded text-zinc-300">{b.filename}</code>
                    </td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{b.workload_count}</td>
                    <td className="py-3 px-4 text-sm text-zinc-400">{formatTimestamp(b.created_at)}</td>
                    <td className="py-3 px-4 text-sm text-zinc-400">{b.aether_version}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
