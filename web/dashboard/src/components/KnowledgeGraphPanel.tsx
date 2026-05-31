// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { GitBranch, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { pathWithQuery } from '../utils/urlState';
import { viewToPath } from '../utils/dashboardRoutes';
import type { KnowledgeGraphReport } from '../types/api';
import GlassSection from './GlassSection';

const KIND_COLORS: Record<string, string> = {
  workload: 'rgb(99 164 255)',
  runtime: 'rgb(168 85 247)',
  cluster: 'rgb(45 212 191)',
  threat: 'rgb(248 113 113)',
  drift: 'rgb(251 191 36)',
};

const EDGE_KINDS = ['depends_on', 'runs_on', 'placed_on', 'threatens', 'drifts_from'];

export default function KnowledgeGraphPanel() {
  const navigate = useNavigate();
  const [graph, setGraph] = useState<KnowledgeGraphReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [edgeFilter, setEdgeFilter] = useState<string[]>([]);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const dragRef = useRef<{ x: number; y: number } | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const qs =
      edgeFilter.length > 0
        ? `?edge_kinds=${encodeURIComponent(edgeFilter.join(','))}`
        : '';
    const data = await apiFetch<KnowledgeGraphReport>(`/intelligence/graph/interactive${qs}`);
    setGraph(data);
    setLoading(false);
  }, [edgeFilter]);

  useEffect(() => {
    void load();
  }, [load]);

  const layout = useMemo(() => {
    if (!graph) return { width: 800, height: 360, positions: new Map<string, { x: number; y: number }>() };
    const byKind = new Map<string, typeof graph.nodes>();
    for (const node of graph.nodes) {
      const list = byKind.get(node.kind) ?? [];
      list.push(node);
      byKind.set(node.kind, list);
    }
    const positions = new Map<string, { x: number; y: number }>();
    const kinds = ['workload', 'runtime', 'cluster', 'threat', 'drift'];
    let x = 80;
    for (const kind of kinds) {
      const group = byKind.get(kind) ?? [];
      group.forEach((node, i) => {
        positions.set(node.id, { x, y: 50 + i * 56 });
      });
      x += 150;
    }
    const height = Math.max(360, (Math.max(...Array.from(byKind.values()).map((g) => g.length), 0) + 1) * 56);
    return { width: 820, height, positions };
  }, [graph]);

  function handleNodeClick(nodeId: string) {
    if (nodeId.startsWith('wl:')) {
      navigate(pathWithQuery(viewToPath('workloads'), { workload: nodeId.slice(3) }));
    }
  }

  return (
    <GlassSection
      accent="purple"
      testId="knowledge-graph-panel"
      title="Infrastructure Knowledge Graph"
      subtitle="Workloads, dependencies, threats, and drift — unified graph for impact analysis."
      icon={<GitBranch className="h-5 w-5 text-teal-400" />}
      actions={
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
        >
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          Refresh
        </button>
      }
    >
      <div className="mb-4 flex flex-wrap gap-2" data-testid="graph-edge-filters">
        {EDGE_KINDS.map((kind) => {
          const active = edgeFilter.includes(kind);
          return (
            <button
              key={kind}
              type="button"
              onClick={() =>
                setEdgeFilter((prev) =>
                  active ? prev.filter((k) => k !== kind) : [...prev, kind],
                )
              }
              className={`rounded-full border px-3 py-1 text-xs ${
                active ? 'border-teal-500/40 bg-teal-500/10 text-teal-200' : 'border-slate-700 text-slate-400'
              }`}
            >
              {kind}
            </button>
          );
        })}
        <button
          type="button"
          onClick={() => setZoom((z) => Math.min(2.5, z + 0.2))}
          className="rounded-full border border-slate-700 px-3 py-1 text-xs text-slate-400"
          data-testid="graph-zoom-in"
        >
          Zoom +
        </button>
        <button
          type="button"
          onClick={() => setZoom((z) => Math.max(0.5, z - 0.2))}
          className="rounded-full border border-slate-700 px-3 py-1 text-xs text-slate-400"
        >
          Zoom −
        </button>
      </div>

      {graph ? (
        <>
          <div className="mb-4 grid gap-3 sm:grid-cols-5">
            <div className="glass-metric-card"><div className="text-xl font-semibold text-white">{graph.stats.workloads}</div><div className="text-xs text-slate-500">Workloads</div></div>
            <div className="glass-metric-card"><div className="text-xl font-semibold text-white">{graph.stats.dependencies}</div><div className="text-xs text-slate-500">Dependencies</div></div>
            <div className="glass-metric-card"><div className="text-xl font-semibold text-white">{graph.stats.threats}</div><div className="text-xs text-slate-500">Threats</div></div>
            <div className="glass-metric-card"><div className="text-xl font-semibold text-white">{graph.stats.drifted}</div><div className="text-xs text-slate-500">Drifted</div></div>
            <div className="glass-metric-card"><div className="text-xl font-semibold text-white">{graph.stats.clusters}</div><div className="text-xs text-slate-500">Clusters</div></div>
          </div>

          {graph.nodes.length === 0 ? (
            <p className="text-sm text-slate-500">Deploy workloads to populate the knowledge graph.</p>
          ) : (
            <div
              className="overflow-auto rounded-2xl border border-slate-800 bg-slate-950/50"
              data-testid="interactive-graph-canvas"
              onMouseDown={(e) => {
                dragRef.current = { x: e.clientX - pan.x, y: e.clientY - pan.y };
              }}
              onMouseMove={(e) => {
                if (!dragRef.current) return;
                setPan({ x: e.clientX - dragRef.current.x, y: e.clientY - dragRef.current.y });
              }}
              onMouseUp={() => {
                dragRef.current = null;
              }}
              onMouseLeave={() => {
                dragRef.current = null;
              }}
            >
              <svg viewBox={`0 0 ${layout.width} ${layout.height}`} className="min-h-[280px] w-full">
                <g transform={`translate(${pan.x} ${pan.y}) scale(${zoom})`}>
                {graph.edges.map((edge, i) => {
                  const a = layout.positions.get(edge.from);
                  const b = layout.positions.get(edge.to);
                  if (!a || !b) return null;
                  return (
                    <line
                      key={`${edge.from}-${edge.to}-${i}`}
                      x1={a.x}
                      y1={a.y}
                      x2={b.x}
                      y2={b.y}
                      stroke="rgb(148 163 184 / 0.35)"
                      strokeWidth={1.2}
                    />
                  );
                })}
                {graph.nodes.map((node) => {
                  const p = layout.positions.get(node.id);
                  if (!p) return null;
                  const color = KIND_COLORS[node.kind] ?? 'rgb(148 163 184)';
                  return (
                    <g
                      key={node.id}
                      className={node.kind === 'workload' ? 'cursor-pointer' : undefined}
                      onClick={() => handleNodeClick(node.id)}
                    >
                      <circle cx={p.x} cy={p.y} r={16} fill={color} fillOpacity={0.2} stroke={color} strokeWidth={1.5} />
                      <text x={p.x} y={p.y + 28} textAnchor="middle" className="fill-slate-300 text-[10px]">
                        {node.label.length > 14 ? `${node.label.slice(0, 12)}…` : node.label}
                      </text>
                    </g>
                  );
                })}
                </g>
              </svg>
            </div>
          )}
        </>
      ) : loading ? (
        <div className="flex items-center gap-2 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" />
          Building knowledge graph…
        </div>
      ) : null}
    </GlassSection>
  );
}
