import { withAuroraPage } from '../layout/AuroraPage';
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
      className="text-primary hover:underline"
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
              className={`quick-link-chip rounded-full px-3 py-1.5 text-sm text-foreground ${
                highlightWorkload === node ? 'border-primary/60 ring-1 ring-aether/30' : 'glass-divider'
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
              className="glass flex items-center gap-2 text-sm px-4 py-2"
            >
              <span className="font-medium text-foreground"><WorkloadNodeLink name={edge.from} /></span>
              <ArrowRight size={14} className="text-primary shrink-0" />
              <span className="text-muted">depends on</span>
              <span className="font-medium text-primary"><WorkloadNodeLink name={edge.to} /></span>
              {onRemove && (
                <button
                  type="button"
                  disabled={removeBusy === `${edge.from}->${edge.to}`}
                  onClick={() => onRemove(edge.from, edge.to)}
                  className="ml-auto p-1 text-subtle hover:text-danger"
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
            {i > 0 && <ArrowRight size={14} className="text-subtle shrink-0 -ml-1" />}
            <div className="glass flex items-center gap-3 flex-1 px-4 py-2">
              <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold text-primary">
                {i + 1}
              </span>
              <span
                className={`text-sm text-foreground ${highlightWorkload === name ? 'text-primary font-medium' : ''}`}
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

  return <p className="text-sm text-subtle">No graph nodes to display.</p>;
}

function DepsPage({ refreshKey }: { refreshKey?: number } = {}) {
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
  }, [refreshKey]);

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
          className="text-xs text-primary hover:underline"
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
          className="text-xs text-primary hover:underline ml-3"
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
            className="text-primary hover:underline"
            data-testid="deps-context-compose-link"
          >
            Compose →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('envs'), { workload: highlightWorkload })}
            className="text-primary hover:underline"
            data-testid="deps-envs-scoped-link"
          >
            Environments →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: highlightWorkload })}
            className="text-primary hover:underline"
            data-testid="deps-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: highlightWorkload })}
            className="text-primary hover:underline"
            data-testid="deps-context-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: highlightWorkload })}
            className="text-primary hover:underline"
            data-testid="deps-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: highlightWorkload })}
            className="text-primary hover:underline"
            data-testid="deps-context-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('zyra'), { workload: highlightWorkload, q: `Dependency graph for ${highlightWorkload}` })}
            className="text-primary hover:underline"
            data-testid="deps-context-copilot-link"
          >
            Copilot →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <div className="glass mb-6">
        <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
          <Plus size={20} className="text-success" />
          Add dependency
        </h2>
        <div className="flex flex-wrap items-end gap-3" data-testid="deps-add-form">
          <div>
            <label className="block text-xs text-muted mb-1">Workload</label>
            <input
              type="text"
              value={addWorkload}
              onChange={(e) => setAddWorkload(e.target.value)}
              placeholder="e.g. web-app"
              className="glass-input text-foreground placeholder-slate-600 focus:outline-none focus:border-primary"
            />
          </div>
          <div>
            <label className="block text-xs text-muted mb-1">Depends on</label>
            <input
              type="text"
              value={addDependency}
              onChange={(e) => setAddDependency(e.target.value)}
              placeholder="e.g. database"
              className="glass-input text-foreground placeholder-slate-600 focus:outline-none focus:border-primary"
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
          <section className="mb-10" data-testid="deps-stats-panel">
            <div className="flex flex-wrap gap-x-10 gap-y-4">
              <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{graph.stats.total_workloads}</div>Workloads</div>
              <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{graph.stats.total_edges}</div>Edges</div>
              <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{graph.stats.root_workloads}</div>Root</div>
              <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{graph.stats.leaf_workloads}</div>Leaf</div>
              <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{graph.stats.max_depth}</div>Max depth</div>
              <div className="text-sm text-muted"><div className="text-2xl font-semibold tabular-nums text-foreground">{graph.stats.has_cycles ? 'Yes' : 'No'}</div>Cycles</div>
            </div>
          </section>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="glass lg:col-span-2" data-testid="deps-graph-panel">
              <h2 className="text-lg font-semibold text-foreground mb-4">Dependency graph</h2>
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

            <div className="glass" data-testid="deps-issues-panel">
              <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
                <h2 className="text-lg font-semibold text-foreground">Issues</h2>
                <button
                  type="button"
                  data-testid="deps-scheduler-link"
                  onClick={() => navigate(viewToPath('scheduler'))}
                  className="text-xs text-primary hover:underline"
                >
                  Placement scheduler →
                </button>
              </div>
              {graph.issues.length === 0 ? (
                <div className="flex items-center gap-2">
                  <Badge text="No issues" variant="green" />
                  <span className="text-sm text-muted">Dependency graph is clean</span>
                </div>
              ) : (
                <div className="space-y-2">
                  {graph.issues.map((issue, i) => (
                    <div key={i} className="flex items-start gap-2 p-3 bg-danger/5 border border-danger/20 rounded-lg">
                      <Badge text="Issue" variant="red" />
                      <span className="text-sm text-muted">{issue}</span>
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

export default withAuroraPage('deps', DepsPage);