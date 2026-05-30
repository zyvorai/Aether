// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { Box, Cpu, HardDrive, Network, RefreshCw, Zap, ZoomIn, ZoomOut } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { pathWithQuery } from '../utils/urlState';
import { viewToPath } from '../utils/dashboardRoutes';
import type { WorkloadResponse } from '../types/api';
import PageLoading from './PageLoading';
import EmptyState from './EmptyState';

export interface FabricNode {
  id: string;
  label: string;
  kind: 'application' | 'runtime' | 'cluster' | 'node' | 'resource';
  sub?: string;
  workload?: string;
}

export interface FabricEdge {
  from: string;
  to: string;
}

interface FabricGraph {
  nodes: FabricNode[];
  edges: FabricEdge[];
}

const KIND_COLORS: Record<FabricNode['kind'], string> = {
  application: 'rgb(99 164 255)',
  runtime: 'rgb(168 85 247)',
  cluster: 'rgb(45 212 191)',
  node: 'rgb(251 191 36)',
  resource: 'rgb(148 163 184)',
};

function buildFabricGraph(workloads: WorkloadResponse[]): FabricGraph {
  const nodes: FabricNode[] = [];
  const edges: FabricEdge[] = [];
  const seen = new Set<string>();

  const addNode = (node: FabricNode) => {
    if (seen.has(node.id)) return;
    seen.add(node.id);
    nodes.push(node);
  };

  const addEdge = (from: string, to: string) => {
    edges.push({ from, to });
  };

  for (const w of workloads) {
    const appId = `app:${w.name}`;
    addNode({
      id: appId,
      label: w.name,
      kind: 'application',
      sub: w.kind ?? 'Workload',
      workload: w.name,
    });

    const runtimeId = `rt:${w.name}:${w.runtime}`;
    addNode({
      id: runtimeId,
      label: w.runtime,
      kind: 'runtime',
      sub: 'Runtime',
      workload: w.name,
    });
    addEdge(appId, runtimeId);

    const clusterName = w.cluster ?? 'local';
    const clusterId = `cluster:${clusterName}`;
    addNode({
      id: clusterId,
      label: clusterName,
      kind: 'cluster',
      sub: 'Cluster',
    });
    addEdge(runtimeId, clusterId);

    const nodeId = `node:${clusterName}:${w.namespace ?? 'default'}`;
    addNode({
      id: nodeId,
      label: w.namespace ? `ns/${w.namespace}` : 'Node pool',
      kind: 'node',
      sub: 'Placement',
      workload: w.name,
    });
    addEdge(clusterId, nodeId);

    for (const [res, icon] of [
      ['CPU', 'cpu'],
      ['Memory', 'mem'],
      ['Storage', 'disk'],
    ] as const) {
      const resId = `res:${w.name}:${icon}`;
      addNode({ id: resId, label: res, kind: 'resource', sub: icon.toUpperCase(), workload: w.name });
      addEdge(nodeId, resId);
    }

    if (w.runtime.toLowerCase().includes('virt') || w.runtime.toLowerCase().includes('metal')) {
      const gpuId = `res:${w.name}:gpu`;
      addNode({ id: gpuId, label: 'GPU', kind: 'resource', sub: 'Accelerator', workload: w.name });
      addEdge(nodeId, gpuId);
    }
  }

  return { nodes, edges };
}

interface RuntimeFabricGraphProps {
  workloads: WorkloadResponse[];
  onSelectWorkload?: (name: string) => void;
}

export default function RuntimeFabricGraph({ workloads, onSelectWorkload }: RuntimeFabricGraphProps) {
  const graph = useMemo(() => buildFabricGraph(workloads), [workloads]);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [selected, setSelected] = useState<string | null>(null);

  const layout = useMemo(() => {
    const positions = new Map<string, { x: number; y: number }>();
    const byKind: Record<FabricNode['kind'], FabricNode[]> = {
      application: [],
      runtime: [],
      cluster: [],
      node: [],
      resource: [],
    };
    for (const n of graph.nodes) byKind[n.kind].push(n);

    let y = 40;
    const colX: Record<FabricNode['kind'], number> = {
      application: 80,
      runtime: 220,
      cluster: 360,
      node: 500,
      resource: 640,
    };

    for (const kind of ['application', 'runtime', 'cluster', 'node', 'resource'] as const) {
      const group = byKind[kind];
      const startY = y;
      group.forEach((n, i) => {
        positions.set(n.id, { x: colX[kind], y: startY + i * 72 });
      });
      if (group.length > 0) y = Math.max(y, startY + group.length * 72 + 24);
    }

    const width = 760;
    const height = Math.max(320, y + 40);
    return { positions, width, height };
  }, [graph.nodes]);

  const handleNodeClick = (node: FabricNode) => {
    setSelected(node.id);
    if (node.workload) onSelectWorkload?.(node.workload);
  };

  if (graph.nodes.length === 0) {
    return (
      <EmptyState
        icon={<Box size={40} />}
        title="No fabric topology yet"
        description="Deploy workloads to see the live Application → Runtime → Cluster → Node graph."
      />
    );
  }

  return (
    <div className="space-y-3" data-testid="runtime-fabric-graph">
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => setZoom((z) => Math.min(2, z + 0.1))}
          className="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-400 hover:text-white"
          aria-label="Zoom in"
        >
          <ZoomIn className="h-4 w-4" />
        </button>
        <button
          type="button"
          onClick={() => setZoom((z) => Math.max(0.5, z - 0.1))}
          className="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-400 hover:text-white"
          aria-label="Zoom out"
        >
          <ZoomOut className="h-4 w-4" />
        </button>
        <button
          type="button"
          onClick={() => {
            setZoom(1);
            setPan({ x: 0, y: 0 });
          }}
          className="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-400 hover:text-white"
        >
          Reset view
        </button>
        <span className="ml-auto text-xs text-slate-500">{graph.nodes.length} nodes · live</span>
      </div>

      <div className="overflow-hidden rounded-[24px] border border-slate-800/60 bg-slate-950/60 backdrop-blur-xl">
        <svg
          viewBox={`${-pan.x} ${-pan.y} ${layout.width / zoom} ${layout.height / zoom}`}
          className="w-full min-h-[360px] fabric-graph-svg"
          role="img"
          aria-label="Runtime fabric topology"
        >
          <defs>
            <linearGradient id="fabric-edge" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stopColor="rgba(99,164,255,0.2)" />
              <stop offset="100%" stopColor="rgba(168,85,247,0.5)" />
            </linearGradient>
            <filter id="fabric-glow">
              <feGaussianBlur stdDeviation="2" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {graph.edges.map((e, i) => {
            const a = layout.positions.get(e.from);
            const b = layout.positions.get(e.to);
            if (!a || !b) return null;
            return (
              <line
                key={`${e.from}-${e.to}-${i}`}
                x1={a.x}
                y1={a.y}
                x2={b.x}
                y2={b.y}
                stroke="url(#fabric-edge)"
                strokeWidth={1.5}
                className="fabric-edge-animate"
              />
            );
          })}

          {graph.nodes.map((node) => {
            const p = layout.positions.get(node.id);
            if (!p) return null;
            const active = selected === node.id;
            const color = KIND_COLORS[node.kind];
            return (
              <g
                key={node.id}
                transform={`translate(${p.x}, ${p.y})`}
                onClick={() => handleNodeClick(node)}
                className="cursor-pointer"
                role="button"
                tabIndex={0}
                onKeyDown={(ev) => {
                  if (ev.key === 'Enter' || ev.key === ' ') handleNodeClick(node);
                }}
              >
                <rect
                  x={-52}
                  y={-22}
                  width={104}
                  height={44}
                  rx={12}
                  fill="rgba(15,23,42,0.92)"
                  stroke={active ? color : `${color.replace('rgb', 'rgba').replace(')', ', 0.55)')}`}
                  strokeWidth={active ? 2.5 : 1.5}
                  filter={active ? 'url(#fabric-glow)' : undefined}
                />
                <text textAnchor="middle" y={-2} className="fill-slate-100 text-[11px] font-medium">
                  {node.label.length > 12 ? `${node.label.slice(0, 10)}…` : node.label}
                </text>
                {node.sub ? (
                  <text textAnchor="middle" y={12} className="fill-slate-500 text-[9px]">
                    {node.sub}
                  </text>
                ) : null}
              </g>
            );
          })}
        </svg>
      </div>

      <div className="flex flex-wrap gap-3 text-[10px] uppercase tracking-[0.14em] text-slate-500">
        <span className="inline-flex items-center gap-1"><Cpu className="h-3 w-3 text-sky-400" /> Application</span>
        <span className="inline-flex items-center gap-1"><Network className="h-3 w-3 text-violet-400" /> Runtime</span>
        <span className="inline-flex items-center gap-1"><HardDrive className="h-3 w-3 text-teal-400" /> Cluster</span>
        <span className="inline-flex items-center gap-1"><Zap className="h-3 w-3 text-amber-400" /> Resources</span>
      </div>
    </div>
  );
}

export function FabricPageContent() {
  const navigate = useNavigate();
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<WorkloadResponse[]>('/workloads');
    setWorkloads(data ?? []);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading) return <PageLoading label="Loading runtime fabric…" />;

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-aether">Runtime Fabric</p>
          <h2 className="mt-2 text-2xl font-semibold text-white">Live infrastructure topology</h2>
          <p className="mt-2 max-w-2xl text-sm text-slate-400">
            Application → Runtime → Cluster → Node → CPU / GPU / Storage. Click a workload to drill down.
          </p>
        </div>
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-sm text-slate-300 hover:border-aether/40"
        >
          <RefreshCw className="h-4 w-4" />
          Refresh
        </button>
      </div>
      <RuntimeFabricGraph
        workloads={workloads}
        onSelectWorkload={(name) => navigate(pathWithQuery(viewToPath('workloads'), { workload: name }))}
      />
    </div>
  );
}
