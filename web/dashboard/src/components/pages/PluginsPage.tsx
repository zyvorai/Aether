import { useState, useEffect } from 'react';
import { Search, Inbox } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import CodeBlock from '../CodeBlock';
import Modal from '../Modal';
import type { PluginInfo } from '../../types/api';

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [discovering, setDiscovering] = useState(false);
  const [discoverResult, setDiscoverResult] = useState<string | null>(null);
  const [selectedPlugin, setSelectedPlugin] = useState<PluginInfo | null>(null);
  const [runtimeFilter, setRuntimeFilter] = useState('all');

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

  const runtimeOptions = ['all', ...Array.from(new Set(plugins.map((plugin) => plugin.runtime_kind)))];
  const visiblePlugins = runtimeFilter === 'all'
    ? plugins
    : plugins.filter((plugin) => plugin.runtime_kind === runtimeFilter);

  return (
    <div>
      <div className="mb-6 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex items-center gap-2">
          <label className="text-xs uppercase tracking-[0.18em] text-slate-500">Runtime</label>
          <select
            value={runtimeFilter}
            onChange={(e) => setRuntimeFilter(e.target.value)}
            className="rounded-lg border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm text-slate-200"
          >
            {runtimeOptions.map((option) => (
              <option key={option} value={option}>
                {option === 'all' ? 'All runtimes' : option}
              </option>
            ))}
          </select>
        </div>
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
        <div className="mb-6 dash-card">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">Discovery Result</h2>
          <CodeBlock title="json">{discoverResult}</CodeBlock>
        </div>
      )}

      {visiblePlugins.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No plugins" description="No plugins are installed. Try discovering plugins." />
      ) : (
        <div className="dash-card-flush">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Version</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Runtime</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Capabilities</th>
                  <th className="text-right text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {visiblePlugins.map((p) => (
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
                    <td className="py-3 px-4 text-right">
                      <button
                        onClick={() => setSelectedPlugin(p)}
                        className="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 transition-colors hover:bg-slate-800"
                      >
                        Inspect
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal
        isOpen={selectedPlugin !== null}
        onClose={() => setSelectedPlugin(null)}
        title={selectedPlugin ? `Plugin: ${selectedPlugin.name}` : 'Plugin'}
      >
        {selectedPlugin && (
          <div className="space-y-4">
            <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">Runtime</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.runtime_kind}</div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">Version</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.version}</div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">Capabilities</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.capabilities.length}</div>
              </div>
            </div>
            <div>
              <div className="mb-2 text-sm font-medium text-slate-200">Command</div>
              <CodeBlock title="command">{selectedPlugin.command}</CodeBlock>
            </div>
            <div>
              <div className="mb-2 text-sm font-medium text-slate-200">Capabilities</div>
              <div className="flex flex-wrap gap-2">
                {selectedPlugin.capabilities.map((capability) => (
                  <Badge key={capability} text={capability} variant="blue" />
                ))}
              </div>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
