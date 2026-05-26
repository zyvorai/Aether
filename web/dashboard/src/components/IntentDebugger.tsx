import { useState } from 'react';
import { apiPost } from '../utils/api';
import RadarChart from './RadarChart';

interface ScoreEntry {
  runtime: string;
  total_score?: number;
  cost?: number;
  cost_score?: number;
  performance?: number;
  performance_score?: number;
  reliability?: number;
  reliability_score?: number;
  availability?: number;
  availability_score?: number;
  reasons?: string[];
  warnings?: string[];
}

interface AnalysisResult {
  recommended?: string;
  recommended_runtime?: string;
  scores?: ScoreEntry[];
  confidence?: number;
  workload_class?: string;
  explain?: boolean;
}

export default function IntentDebugger() {
  const [result, setResult] = useState<AnalysisResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const analyze = async () => {
    setLoading(true);
    setError('');
    try {
      const resp = await apiPost<AnalysisResult>('/ai/recommend', { explain: true });
      if (resp.success && resp.data) {
        setResult(resp.data);
      } else {
        setError(resp.error || 'Analysis failed');
      }
    } catch {
      setError('Failed to connect to API');
    } finally {
      setLoading(false);
    }
  };

  if (!result) {
    return (
      <div className="dash-card text-center">
        <h3 className="text-lg font-bold text-white mb-2">Intent Debugger</h3>
        <p className="text-zinc-400 text-sm mb-4">
          Per-runtime placement analysis with reasons and warnings for every candidate runtime.
        </p>
        <button
          onClick={analyze}
          disabled={loading}
          className="px-4 py-2 bg-aether text-white rounded-lg hover:bg-aether-light transition-colors disabled:opacity-50"
        >
          {loading ? 'Analyzing...' : 'Run Analysis'}
        </button>
        {error && <p className="text-red-400 text-sm mt-2">{error}</p>}
      </div>
    );
  }

  const recommended = result.recommended || result.recommended_runtime;
  const scores = result.scores || [];
  const confidence = result.confidence || 0;
  const workloadClass = result.workload_class || 'Unknown';

  const recScore = scores.find((s) => s.runtime === recommended);
  const radarDims = recScore
    ? [
        { label: 'Cost', value: Math.min(recScore.cost ?? recScore.cost_score ?? 0, 1) },
        { label: 'Perf', value: Math.min(recScore.performance ?? recScore.performance_score ?? 0, 1) },
        { label: 'Reliability', value: Math.min(recScore.reliability ?? recScore.reliability_score ?? 0, 1) },
        { label: 'Availability', value: Math.min(recScore.availability ?? recScore.availability_score ?? 0, 1) },
      ]
    : [];

  return (
    <div className="dash-card-flush">
      <div className="px-4 py-3 bg-zinc-800 border-b border-zinc-700 flex items-center justify-between">
        <h3 className="text-lg font-bold text-white">Intent Debugger</h3>
        <button onClick={analyze} className="text-xs text-aether hover:text-aether-light">Re-analyze</button>
      </div>

      <div className="p-4 grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <div className="mb-3">
            <span className="text-zinc-500 text-xs">RECOMMENDED RUNTIME</span>
            <p className="text-2xl font-bold text-aether">{recommended}</p>
          </div>
          <div className="mb-3">
            <span className="text-zinc-500 text-xs">WORKLOAD CLASS</span>
            <p className="text-white">{workloadClass}</p>
          </div>
          <div className="mb-3">
            <span className="text-zinc-500 text-xs">CONFIDENCE</span>
            <div className="flex items-center gap-2">
              <div className="flex-1 bg-zinc-700 rounded-full h-2">
                <div className="bg-aether h-2 rounded-full" style={{ width: `${confidence * 100}%` }} />
              </div>
              <span className="text-white text-sm">{(confidence * 100).toFixed(0)}%</span>
            </div>
          </div>
        </div>

        <div className="flex flex-col items-center">
          {radarDims.length >= 3 && <RadarChart dimensions={radarDims} />}

          <div className="w-full mt-3">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-zinc-500 text-xs">
                  <th className="text-left py-1">Runtime</th>
                  <th className="text-right">Score</th>
                  <th className="text-right">Cost</th>
                  <th className="text-right">Perf</th>
                </tr>
              </thead>
              <tbody>
                {scores.map((s) => (
                  <tr key={s.runtime} className={s.runtime === recommended ? 'text-aether' : 'text-zinc-400'}>
                    <td className="py-0.5">{s.runtime === recommended ? '> ' : '  '}{s.runtime}</td>
                    <td className="text-right">{((s.total_score ?? 0) * 100).toFixed(0)}%</td>
                    <td className="text-right">{((s.cost ?? s.cost_score ?? 0) * 100).toFixed(0)}%</td>
                    <td className="text-right">{((s.performance ?? s.performance_score ?? 0) * 100).toFixed(0)}%</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <div className="px-4 pb-4 space-y-3 border-t border-zinc-700 pt-4">
        <span className="text-zinc-500 text-xs">PER-RUNTIME REASONS</span>
        {scores.map((s) => (
          <div
            key={s.runtime}
            className={`rounded-lg border p-3 ${s.runtime === recommended ? 'border-aether/40 bg-aether/5' : 'border-zinc-700 bg-zinc-900/50'}`}
          >
            <p className="text-sm font-medium text-white mb-2">{s.runtime}</p>
            {(s.reasons ?? []).map((r, i) => (
              <div key={`r-${i}`} className="flex items-start gap-2 mt-1">
                <span className="text-emerald-400">+</span>
                <span className="text-zinc-300 text-sm">{r}</span>
              </div>
            ))}
            {(s.warnings ?? []).map((w, i) => (
              <div key={`w-${i}`} className="flex items-start gap-2 mt-1">
                <span className="text-orange-400">−</span>
                <span className="text-zinc-300 text-sm">{w}</span>
              </div>
            ))}
            {!(s.reasons?.length || s.warnings?.length) && (
              <p className="text-zinc-500 text-sm">No specific factors recorded.</p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
