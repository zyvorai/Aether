import { useState, useEffect } from 'react';
import { Inbox, Plus } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import StatCard from '../StatCard';
import Badge from '../Badge';
import CodeBlock from '../CodeBlock';
import EmptyState from '../EmptyState';
import type { DependencyGraph } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function DepsPage() {
  const [graph, setGraph] = useState<DependencyGraph | null>(null);
  const [loading, setLoading] = useState(true);
  const [addWorkload, setAddWorkload] = useState('');
  const [addDependency, setAddDependency] = useState('');
  const [addLoading, setAddLoading] = useState(false);

  async function load() {
    setLoading(true);
    const data = await apiFetch<DependencyGraph>('/dependencies');
    setGraph(data);
    setLoading(false);
  }

  useEffect(() => { load(); }, []);

  async function handleAddDependency() {
    if (!addWorkload.trim() || !addDependency.trim()) return;
    setAddLoading(true);
    const res = await apiPost('/dependencies', {
      workload: addWorkload.trim(),
      dependency: addDependency.trim(),
    });
    setAddLoading(false);
    if (res.success) {
      toast(`Dependency "${addWorkload.trim()}" -> "${addDependency.trim()}" added`, 'success');
      setAddWorkload('');
      setAddDependency('');
      load();
    } else {
      toast(`Failed to add dependency: ${res.error ?? 'unknown error'}`, 'error');
    }
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
      {/* Add Dependency Form */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 mb-6">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4 flex items-center gap-2">
          <Plus size={20} className="text-emerald-400" />
          Add Dependency
        </h2>
        <div className="flex flex-wrap items-end gap-3">
          <div>
            <label className="block text-xs text-zinc-400 mb-1">Workload</label>
            <input
              type="text"
              value={addWorkload}
              onChange={(e) => setAddWorkload(e.target.value)}
              placeholder="e.g. web-app"
              className="px-3 py-2 bg-zinc-950 border border-zinc-800 rounded-lg text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-amber-500"
            />
          </div>
          <div>
            <label className="block text-xs text-zinc-400 mb-1">Depends On</label>
            <input
              type="text"
              value={addDependency}
              onChange={(e) => setAddDependency(e.target.value)}
              placeholder="e.g. database"
              className="px-3 py-2 bg-zinc-950 border border-zinc-800 rounded-lg text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-amber-500"
            />
          </div>
          <button
            onClick={handleAddDependency}
            disabled={addLoading || !addWorkload.trim() || !addDependency.trim()}
            className="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
          >
            <Plus size={16} />
            {addLoading ? 'Adding...' : 'Add'}
          </button>
        </div>
      </div>

      {!graph ? (
        <EmptyState icon={<Inbox size={48} />} title="No dependency data" description="No dependency graph available" />
      ) : (
        <>
          <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
            <StatCard title="Workloads" value={graph.stats.total_workloads} color="blue" />
            <StatCard title="Edges" value={graph.stats.total_edges} color="purple" />
            <StatCard title="Root" value={graph.stats.root_workloads} color="green" />
            <StatCard title="Leaf" value={graph.stats.leaf_workloads} color="yellow" />
            <StatCard title="Max Depth" value={graph.stats.max_depth} color="orange" />
            <StatCard
              title="Cycles"
              value={graph.stats.has_cycles ? 'Yes' : 'No'}
              color={graph.stats.has_cycles ? 'red' : 'green'}
            />
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            {/* Startup Order */}
            <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
              <h2 className="text-lg font-semibold text-zinc-100 mb-4">Startup Order</h2>
              {graph.startup_order.length === 0 ? (
                <p className="text-sm text-zinc-500">No startup order defined</p>
              ) : (
                <div className="space-y-2">
                  {graph.startup_order.map((name, i) => (
                    <div key={name} className="flex items-center gap-3 p-2 bg-zinc-950/50 rounded-lg">
                      <span className="flex items-center justify-center w-6 h-6 rounded-full bg-amber-500/10 text-amber-400 text-xs font-bold">
                        {i + 1}
                      </span>
                      <span className="text-sm text-zinc-200">{name}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Issues */}
            <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
              <h2 className="text-lg font-semibold text-zinc-100 mb-4">Issues</h2>
              {graph.issues.length === 0 ? (
                <div className="flex items-center gap-2">
                  <Badge text="No Issues" variant="green" />
                  <span className="text-sm text-zinc-400">Dependency graph is clean</span>
                </div>
              ) : (
                <div className="space-y-2">
                  {graph.issues.map((issue, i) => (
                    <div key={i} className="flex items-start gap-2 p-3 bg-red-500/5 border border-red-500/20 rounded-lg">
                      <Badge text="Issue" variant="red" />
                      <span className="text-sm text-zinc-300">{issue}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Full Graph JSON */}
          <div className="mt-6 bg-zinc-900 border border-zinc-800 rounded-xl p-6">
            <h2 className="text-lg font-semibold text-zinc-100 mb-4">Full Graph</h2>
            <CodeBlock title="json">{JSON.stringify(graph, null, 2)}</CodeBlock>
          </div>
        </>
      )}
    </div>
  );
}
