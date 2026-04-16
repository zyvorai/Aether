import { useState, useEffect } from 'react';
import { Search, Inbox } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import CodeBlock from '../CodeBlock';
import type { PluginInfo } from '../../types/api';

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [discovering, setDiscovering] = useState(false);
  const [discoverResult, setDiscoverResult] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    const data = await apiFetch<PluginInfo[]>('/plugins');
    setPlugins(data ?? []);
    setLoading(false);
  }

  useEffect(() => { load(); }, []);

  async function handleDiscover() {
    setDiscovering(true);
    const res = await apiPost<unknown>('/plugins/discover');
    setDiscoverResult(JSON.stringify(res.data ?? res, null, 2));
    setDiscovering(false);
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
          onClick={handleDiscover}
          disabled={discovering}
          className="flex items-center gap-2 px-4 py-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
        >
          <Search size={16} />
          {discovering ? 'Discovering...' : 'Discover Plugins'}
        </button>
      </div>

      {discoverResult && (
        <div className="mb-6 bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Discovery Result</h2>
          <CodeBlock title="json">{discoverResult}</CodeBlock>
        </div>
      )}

      {plugins.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No plugins" description="No plugins are installed. Try discovering plugins." />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Version</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Runtime</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Capabilities</th>
                </tr>
              </thead>
              <tbody>
                {plugins.map((p) => (
                  <tr key={p.name} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-4 font-medium text-zinc-200">{p.name}</td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{p.version}</td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{p.runtime_kind}</td>
                    <td className="py-3 px-4">
                      <div className="flex flex-wrap gap-1">
                        {p.capabilities.map((c) => (
                          <Badge key={c} text={c} variant="blue" />
                        ))}
                      </div>
                    </td>
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
