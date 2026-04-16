import { useState, useEffect } from 'react';
import { RefreshCw } from 'lucide-react';
import { apiText } from '../../utils/api';
import CodeBlock from '../CodeBlock';

export default function MetricsPage() {
  const [metrics, setMetrics] = useState('');
  const [loading, setLoading] = useState(true);

  async function load() {
    setLoading(true);
    const text = await apiText('/metrics');
    setMetrics(text);
    setLoading(false);
  }

  useEffect(() => { load(); }, []);

  return (
    <div>
      <div className="flex items-center justify-end mb-6">
        <button
          onClick={load}
          disabled={loading}
          className="flex items-center gap-2 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 disabled:opacity-50 text-zinc-200 rounded-lg text-sm font-medium transition-colors"
        >
          <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
          Refresh
        </button>
      </div>

      <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4">Prometheus Metrics</h2>
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
          </div>
        ) : (
          <CodeBlock title="prometheus">{metrics}</CodeBlock>
        )}
      </div>
    </div>
  );
}
