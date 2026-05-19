import { useState, useEffect, useCallback, useMemo } from 'react';
import { Search, Inbox } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import Modal from '../Modal';
import type { PluginInfo } from '../../types/api';

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [discovering, setDiscovering] = useState(false);
  const [discoverSummary, setDiscoverSummary] = useState<string | null>(null);
  const [selectedPlugin, setSelectedPlugin] = useState<PluginInfo | null>(null);
  const [search, setSearch] = useState('');
  const [runtimeFilter, setRuntimeFilter] = useState('all');

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<PluginInfo[]>('/plugins');
    setPlugins(data ?? []);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleDiscover() {
    setDiscovering(true);
    const res = await apiPost<{ discovered?: number; plugins?: string[]; message?: string }>('/plugins/discover');
    setDiscovering(false);
    if (res.success && res.data) {
      const count = res.data.discovered ?? (Array.isArray(res.data.plugins) ? res.data.plugins.length : null);
      setDiscoverSummary(
        count != null ? `Discovered ${count} plugin(s)` : (res.data.message ?? 'Discovery completed')
      );
    } else {
      setDiscoverSummary(res.error ?? 'Discovery failed');
    }
    void load();
  }

  const runtimeOptions = useMemo(
    () => ['all', ...Array.from(new Set(plugins.map((plugin) => plugin.runtime_kind)))],
    [plugins]
  );

  const visiblePlugins = useMemo(() => {
    const q = search.trim().toLowerCase();
    return plugins.filter((plugin) => {
      if (runtimeFilter !== 'all' && plugin.runtime_kind !== runtimeFilter) return false;
      if (!q) return true;
      return (
        plugin.name.toLowerCase().includes(q) ||
        plugin.runtime_kind.toLowerCase().includes(q) ||
        plugin.capabilities.some((c) => c.toLowerCase().includes(q))
      );
    });
  }, [plugins, runtimeFilter, search]);

  if (loading && plugins.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-aether" />
      </div>
    );
  }

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search plugins…"
        onRefresh={() => void load()}
        refreshing={loading || discovering}
        filters={
          <select
            value={runtimeFilter}
            onChange={(e) => setRuntimeFilter(e.target.value)}
            className="rounded-xl border border-slate-700/80 bg-slate-950/60 px-3 py-2 text-sm text-slate-200"
            aria-label="Runtime filter"
          >
            {runtimeOptions.map((option) => (
              <option key={option} value={option}>
                {option === 'all' ? 'All runtimes' : option}
              </option>
            ))}
          </select>
        }
        actions={
          <button
            type="button"
            onClick={() => void handleDiscover()}
            disabled={discovering}
            className="inline-flex items-center gap-2 rounded-xl bg-purple-600 px-4 py-2 text-sm font-medium text-white hover:bg-purple-500 disabled:opacity-50"
          >
            <Search size={16} />
            {discovering ? 'Discovering…' : 'Discover'}
          </button>
        }
      />

      {discoverSummary && (
        <div className="dash-card mb-6 text-sm text-slate-300">{discoverSummary}</div>
      )}

      {visiblePlugins.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No plugins" description="No plugins match your filters. Try discovering plugins." />
      ) : (
        <div className="dash-card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-800">
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Version</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Runtime</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Capabilities</th>
                  <th className="text-right text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {visiblePlugins.map((p) => (
                  <tr key={p.name} className="border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors">
                    <td className="py-3 px-4 font-medium text-slate-200">{p.name}</td>
                    <td className="py-3 px-4 text-sm text-slate-300">{p.version}</td>
                    <td className="py-3 px-4 text-sm text-slate-300">{p.runtime_kind}</td>
                    <td className="py-3 px-4">
                      <div className="flex flex-wrap gap-1">
                        {p.capabilities.map((c) => (
                          <Badge key={c} text={c} variant="blue" />
                        ))}
                      </div>
                    </td>
                    <td className="py-3 px-4 text-right">
                      <button
                        type="button"
                        onClick={() => setSelectedPlugin(p)}
                        className="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
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
                <div className="text-xs uppercase tracking-wider text-slate-500">Runtime</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.runtime_kind}</div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Version</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.version}</div>
              </div>
              <div className="rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
                <div className="text-xs uppercase tracking-wider text-slate-500">Capabilities</div>
                <div className="mt-2 text-sm font-medium text-slate-100">{selectedPlugin.capabilities.length}</div>
              </div>
            </div>
            <div>
              <div className="mb-2 text-sm font-medium text-slate-200">Command</div>
              <code className="block rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-sm text-slate-300 font-mono">
                {selectedPlugin.command}
              </code>
            </div>
            <div className="flex flex-wrap gap-2">
              {selectedPlugin.capabilities.map((capability) => (
                <Badge key={capability} text={capability} variant="blue" />
              ))}
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
