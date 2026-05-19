import { useState, useEffect, useCallback, useMemo } from 'react';
import { apiText } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import CodeBlock from '../CodeBlock';

export default function MetricsPage() {
  const [metrics, setMetrics] = useState('');
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    const text = await apiText('/metrics');
    setMetrics(text ?? '');
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filteredMetrics = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return metrics;
    return metrics
      .split('\n')
      .filter((line) => line.startsWith('#') || line.toLowerCase().includes(q))
      .join('\n');
  }, [metrics, search]);

  const lineCount = filteredMetrics.split('\n').filter((l) => l && !l.startsWith('#')).length;

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search metric names…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      <div className="dash-card">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-slate-100">Prometheus metrics</h2>
          {!loading && (
            <span className="text-xs text-slate-500">{lineCount} metric lines</span>
          )}
        </div>
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-aether" />
          </div>
        ) : (
          <CodeBlock title="prometheus">{filteredMetrics || 'No metrics match your search.'}</CodeBlock>
        )}
      </div>
    </div>
  );
}
