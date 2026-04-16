interface RadarChartProps {
  dimensions: { label: string; value: number }[]; // value 0-1
  size?: number;
  color?: string;
}

export default function RadarChart({ dimensions, size = 200, color = '#d35400' }: RadarChartProps) {
  const cx = size / 2;
  const cy = size / 2;
  const r = size * 0.38;
  const n = dimensions.length;
  if (n < 3) return null;

  const angleStep = (2 * Math.PI) / n;

  const getPoint = (i: number, val: number) => ({
    x: cx + r * val * Math.sin(i * angleStep),
    y: cy - r * val * Math.cos(i * angleStep),
  });

  // Grid rings
  const rings = [0.25, 0.5, 0.75, 1.0];

  // Data polygon
  const dataPoints = dimensions.map((_, i) => getPoint(i, dimensions[i].value));
  const dataPath = dataPoints.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ') + ' Z';

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="mx-auto">
      {/* Grid rings */}
      {rings.map(ring => (
        <polygon
          key={ring}
          points={dimensions.map((_, i) => { const p = getPoint(i, ring); return `${p.x},${p.y}`; }).join(' ')}
          fill="none"
          stroke="#374151"
          strokeWidth="1"
        />
      ))}

      {/* Axis lines */}
      {dimensions.map((_, i) => {
        const p = getPoint(i, 1);
        return <line key={i} x1={cx} y1={cy} x2={p.x} y2={p.y} stroke="#374151" strokeWidth="1" />;
      })}

      {/* Data area */}
      <path d={dataPath} fill={color} fillOpacity="0.2" stroke={color} strokeWidth="2" />

      {/* Data dots */}
      {dataPoints.map((p, i) => (
        <circle key={i} cx={p.x} cy={p.y} r="3" fill={color} />
      ))}

      {/* Labels */}
      {dimensions.map((d, i) => {
        const p = getPoint(i, 1.2);
        return (
          <text key={i} x={p.x} y={p.y} textAnchor="middle" dominantBaseline="middle" fill="#9ca3af" fontSize="11">
            {d.label}
          </text>
        );
      })}
    </svg>
  );
}
