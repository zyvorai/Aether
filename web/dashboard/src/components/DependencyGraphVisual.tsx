// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

interface Edge {
  from: string;
  to: string;
}

interface Props {
  nodes: string[];
  edges: Edge[];
  onNodeClick?: (name: string) => void;
  highlightWorkload?: string;
}

const W = 520;
const H = 280;
const R = 22;

export default function DependencyGraphVisual({ nodes, edges, onNodeClick, highlightWorkload }: Props) {
  if (nodes.length === 0) {
    return <p className="text-sm text-ink-3">No dependency nodes.</p>;
  }

  const positions = new Map<string, { x: number; y: number }>();
  const cols = Math.ceil(Math.sqrt(nodes.length));
  nodes.forEach((name, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    positions.set(name, {
      x: 60 + col * ((W - 120) / Math.max(cols - 1, 1)),
      y: 40 + row * ((H - 80) / Math.max(Math.ceil(nodes.length / cols) - 1, 1)),
    });
  });

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="w-full max-h-72 rounded-xl glass-code-block">
      <defs>
        <marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
          <path d="M0,0 L6,3 L0,6 Z" fill="rgb(251 146 60 / 0.8)" />
        </marker>
      </defs>
      {edges.map((e, i) => {
        const a = positions.get(e.from);
        const b = positions.get(e.to);
        if (!a || !b) return null;
        return (
          <line
            key={`${e.from}-${e.to}-${i}`}
            x1={a.x}
            y1={a.y}
            x2={b.x}
            y2={b.y}
            stroke="rgb(251 146 60 / 0.5)"
            strokeWidth={1.5}
            markerEnd="url(#arrow)"
          />
        );
      })}
      {nodes.map((name) => {
        const p = positions.get(name)!;
        const highlighted = highlightWorkload === name;
        return (
          <g
            key={name}
            data-testid={highlighted ? 'deps-workload-highlight' : undefined}
            onClick={() => onNodeClick?.(name)}
            className={onNodeClick ? 'cursor-pointer' : undefined}
            role={onNodeClick ? 'button' : undefined}
            tabIndex={onNodeClick ? 0 : undefined}
            onKeyDown={
              onNodeClick
                ? (e) => {
                    if (e.key === 'Enter' || e.key === ' ') onNodeClick(name);
                  }
                : undefined
            }
          >
            <circle
              cx={p.x}
              cy={p.y}
              r={R}
              fill="rgb(15 23 42)"
              stroke={highlighted ? 'rgb(211 84 0)' : 'rgb(211 84 0 / 0.6)'}
              strokeWidth={highlighted ? 3 : 2}
            />
            <text
              x={p.x}
              y={p.y + 36}
              textAnchor="middle"
              className="fill-slate-300 text-[10px]"
              style={{ fontFamily: 'ui-monospace, monospace' }}
            >
              {name.length > 14 ? `${name.slice(0, 12)}…` : name}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
