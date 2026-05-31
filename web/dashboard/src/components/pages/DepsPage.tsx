// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import { Inbox, Plus, ArrowRight, Trash2 } from 'lucide-react';
import { apiFetchSettled, apiPost, apiDeleteJson } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import PageToolbar from '../PageToolbar';
import StatCard from '../StatCard';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import DependencyGraphVisual from '../DependencyGraphVisual';
import type { DependencyGraph, DependencyEdge } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function WorkloadNodeLink({ name }: { name: string }) {
  return (
    <Link
      to={pathWithQuery(viewToPath('workloads'), { workload: name })}
      className="text-aether hover:underline"
    >
      {name}
    </Link>
  );
}

function GraphVisual({
  graph,
  onRemove,
  removeBusy,
  highlightWorkload,
}: {
  graph: DependencyGraph;
  onRemove?: (from: string, to: string) => void;
  removeBusy?: string | null;
  highlightWorkload?: string;
}) {
  const nodes = graph.nodes ?? graph.startup_order;
  const edges: DependencyEdge[] =
    graph.edges ??
  [];

  if (edges.length > 0) {
    return (
      <div className="space-y-4">
        <div className="flex flex-wrap gap-2">
          {nodes.map((node) => (
            <span
              key={node}
              className={`rounded-full border bg-slate-950/80 px-3 py-1.5 text-sm text-slate-200 ${
                highlightWorkload === node ? 'border-aether/60 ring-1 ring-aether/30' : 'border-slate-700'
              }`}
              data-testid={highlightWorkload === node ? 'deps-workload-highlight' : undefined}
            >
              <WorkloadNodeLink name={node} />
            </span>
          ))}
        </div>
        <ul className="space-y-2">
          {edges.map((edge, i) => (
            <li
              key={`${edge.from}-${edge.to}-${i}`}
              className="flex items-center gap-2 text-sm rounded-xl bg-slate-950/60 border border-slate-800 px-4 py-2"
            >
              <span className="font-medium text-slate-200"><WorkloadNodeLink name={edge.from} /></span>
              <ArrowRight size={14} className="text-aether shrink-0" />
              <span className="text-slate-400">depends on</span>
              <span className="font-medium text-aether"><WorkloadNodeLink name={edge.to} /></span>
              {onRemove && (
                <button
                  type="button"
                  disabled={removeBusy === `${edge.from}->${edge.to}`}
                  onClick={() => onRemove(edge.from, edge.to)}
                  className="ml-auto p-1 text-slate-500 hover:text-red-400"
                  title="Remove dependency"
                >
                  <Trash2 size={14} />
                </button>
              )}
            </li>
          ))}
        </ul>
      </div>
    );
  }

  if (graph.startup_order.length > 0) {
    return (
      <div className="space-y-2">
        {graph.startup_order.map((name, i) => (
          <div key={name} className="flex items-center gap-3">
            {i > 0 && <ArrowRight size={14} className="text-slate-600 shrink-0 -ml-1" />}
            <div className="flex items-center gap-3 flex-1 rounded-xl bg-slate-950/60 border border-slate-800 px-4 py-2">
              <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-aether/10 text-xs font-semibold text-aether">
                {i + 1}
              </span>
              <span
                className={`text-sm text-slate-200 ${highlightWorkload === name ? 'text-aether font-medium' : ''}`}
                data-testid={highlightWorkload === name ? 'deps-workload-highlight' : undefined}
              >
                <WorkloadNodeLink name={name} />
              </span>
            </div>
          </div>
        ))}
      </div>
    );
  }

  return <p className="text-sm text-slate-500">No graph nodes to display.</p>;
}

export default function DepsPage() {
  const navigate = useNavigate();
  const [workloadQuery] = useQueryParam('workload', '');
  const highlightWorkload = workloadQuery.trim() || undefined;
  const [graph, setGraph] = useState<DependencyGraph | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [addWorkload, setAddWorkload] = useState('');
  const [addDependency, setAddDependency] = useState('');
  const [addLoading, setAddLoading] = useState(false);
  const [removeBusy, setRemoveBusy] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<DependencyGraph>('/dependencies');
    if (!result.ok) {
      setLoadFailed(true);
      setGraph(null);
    } else {
      setGraph(result.data);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleAddDependency() {
    if (!addWorkload.trim() || !addDependency.trim()) return;
    setAddLoading(true);
    const res = await apiPost('/dependencies', {
      workload: addWorkload.trim(),
      dependency: addDependency.trim(),
    });
    setAddLoading(false);
    if (res.success) {
      toast(`Dependency "${addWorkload.trim()}" → "${addDependency.trim()}" added`, 'success');
      setAddWorkload('');
      setAddDependency('');
      void load();
    } else {
      toast(`Failed to add dependency: ${res.error ?? 'unknown error'}`, 'error');
    }
  }

  async function handleRemoveDependency(workload: string, dependency: string) {
    setRemoveBusy(`${workload}->${dependency}`);
    const res = await apiDeleteJson('/dependencies', { workload, dependency });
    setRemoveBusy(null);
    if (res.success) {
      toast(`Removed dependency ${workload} → ${dependency}`, 'success');
      void load();
    } else {
      toast(res.error ?? 'Failed to remove dependency', 'error');
    }
  }

  if (loading && !graph && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Dependencies unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />
      <div className="mb-4">
        <button
          type="button"
          data-testid="deps-compose-link"
          onClick={() =>
            navigate(
              highlightWorkload
                ? pathWithQuery(viewToPath('compose'), { workload: highlightWorkload })
                : viewToPath('compose'),
            )
          }
          className="text-xs text-aether hover:underline"
        >
          Compose import →
        </button>
        <button
          type="button"
          data-testid="deps-envs-link"
          onClick={() =>
            navigate(
              highlightWorkload
                ? pathWithQuery(viewToPath('envs'), { workload: highlightWorkload })
                : viewToPath('envs'),
            )
          }
          className="text-xs text-aether hover:underline ml-3"
        >
          Environments →
        </button>
      </div>

      {highlightWorkload ? (
        <WorkloadContextBanner testId="deps-workload-context" workload={highlightWorkload} description="Dependency context">
          <WorkloadScopedCrossLinks workload={highlightWorkload} prefix="deps" showDrift showGitops showAudit />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: highlightWorkload })}
            className="text-aether hover:underline"
            data-testid="deps-context-compose-link"
          >
            Compose →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('envs'), { workload: highlightWorkload })}
            className="text-aether hover:underline"
            data-testid="deps-envs-scoped-link"
          >
            Environments →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: highlightWorkload })}
            className="text-aether hover:underline"
            data-testid="deps-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: highlightWorkload })}
            className="text-aether hover:underline"
            data-testid="deps-context-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: highlightWorkload })}
            className="text-aether hover:underline"
            data-testid="deps-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: highlightWorkload })}
            className="text-aether hover:underline"
            data-testid="deps-context-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('copilot'), { workload: highlightWorkload, q: `Dependency graph for ${highlightWorkload}` })}
            className="text-aether hover:underline"
            data-testid="deps-context-copilot-link"
          >
            Copilot →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <div className="dash-card mb-6">
        <h2 className="text-lg font-semibold text-slate-100 mb-4 flex items-center gap-2">
          <Plus size={20} className="text-emerald-400" />
          Add dependency
        </h2>
        <div className="flex flex-wrap items-end gap-3" data-testid="deps-add-form">
          <div>
            <label className="block text-xs text-slate-400 mb-1">Workload</label>
            <input
              type="text"
              value={addWorkload}
              onChange={(e) => setAddWorkload(e.target.value)}
              placeholder="e.g. web-app"
              className="px-3 py-2 bg-slate-950 border border-slate-700 rounded-xl text-sm text-slate-200 placeholder-slate-600 focus:outline-none focus:border-aether"
            />
          </div>
          <div>
            <label className="block text-xs text-slate-400 mb-1">Depends on</label>
            <input
              type="text"
              value={addDependency}
              onChange={(e) => setAddDependency(e.target.value)}
              placeholder="e.g. database"
              className="px-3 py-2 bg-slate-950 border border-slate-700 rounded-xl text-sm text-slate-200 placeholder-slate-600 focus:outline-none focus:border-aether"
            />
          </div>
          <button
            type="button"
            onClick={() => void handleAddDependency()}
            disabled={addLoading || !addWorkload.trim() || !addDependency.trim()}
            className="flex items-center gap-2 btn-primary disabled:opacity-50"
          >
            <Plus size={16} />
            {addLoading ? 'Adding…' : 'Add'}
          </button>
        </div>
      </div>

      {!graph ? (
        <EmptyState icon={<Inbox size={48} />} title="No dependency data" description="No dependency graph available" />
      ) : (
        <>
          <section className="overview-section-shell mb-6 p-6 sm:p-8">
          <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4" data-testid="deps-stats-panel">
            <StatCard title="Workloads" value={graph.stats.total_workloads} color="blue" />
            <StatCard title="Edges" value={graph.stats.total_edges} color="purple" />
            <StatCard title="Root" value={graph.stats.root_workloads} color="green" />
            <StatCard title="Leaf" value={graph.stats.leaf_workloads} color="yellow" />
            <StatCard title="Max depth" value={graph.stats.max_depth} color="orange" />
            <StatCard
              title="Cycles"
              value={graph.stats.has_cycles ? 'Yes' : 'No'}
              color={graph.stats.has_cycles ? 'red' : 'green'}
            />
          </div>
          </section>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="dash-card lg:col-span-2" data-testid="deps-graph-panel">
              <h2 className="text-lg font-semibold text-slate-100 mb-4">Dependency graph</h2>
              {(graph.edges?.length ?? 0) > 0 && (graph.nodes?.length ?? 0) > 0 ? (
                <DependencyGraphVisual
                  nodes={graph.nodes ?? []}
                  edges={graph.edges ?? []}
                  highlightWorkload={highlightWorkload}
                  onNodeClick={(name) =>
                    navigate(pathWithQuery(viewToPath('workloads'), { workload: name }))
                  }
                />
              ) : (
                <GraphVisual
                  graph={graph}
                  highlightWorkload={highlightWorkload}
                  onRemove={(from, to) => void handleRemoveDependency(from, to)}
                  removeBusy={removeBusy}
                />
              )}
            </div>

            <div className="dash-card" data-testid="deps-issues-panel">
              <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
                <h2 className="text-lg font-semibold text-slate-100">Issues</h2>
                <button
                  type="button"
                  data-testid="deps-scheduler-link"
                  onClick={() => navigate(viewToPath('scheduler'))}
                  className="text-xs text-aether hover:underline"
                >
                  Placement scheduler →
                </button>
              </div>
              {graph.issues.length === 0 ? (
                <div className="flex items-center gap-2">
                  <Badge text="No issues" variant="green" />
                  <span className="text-sm text-slate-400">Dependency graph is clean</span>
                </div>
              ) : (
                <div className="space-y-2">
                  {graph.issues.map((issue, i) => (
                    <div key={i} className="flex items-start gap-2 p-3 bg-red-500/5 border border-red-500/20 rounded-lg">
                      <Badge text="Issue" variant="red" />
                      <span className="text-sm text-slate-300">{issue}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
