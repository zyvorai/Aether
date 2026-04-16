import { useState } from 'react';
import { Inbox } from 'lucide-react';
import { apiPost } from '../../utils/api';
import { formatUSD } from '../../utils/formatters';
import YamlInput from '../YamlInput';
import EmptyState from '../EmptyState';
import type { CostEstimate } from '../../types/api';

export default function CostPage() {
  const [estimates, setEstimates] = useState<CostEstimate[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleEstimate(yaml: string) {
    setLoading(true);
    setError(null);
    const res = await apiPost<CostEstimate[]>('/cost/estimate', { yaml });
    if (res.success && res.data) {
      setEstimates(res.data);
    } else {
      setError(res.error ?? 'Failed to estimate costs');
      setEstimates([]);
    }
    setLoading(false);
  }

  const cheapest = estimates.length > 0
    ? estimates.reduce((a, b) => a.total_monthly < b.total_monthly ? a : b).provider
    : null;

  return (
    <div>
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 mb-6">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4">Workload YAML</h2>
        <YamlInput
          buttonText="Estimate Costs"
          onSubmit={handleEstimate}
          loading={loading}
          placeholder="Paste workload YAML to estimate costs..."
        />
        {error && <p className="mt-2 text-sm text-red-400">{error}</p>}
      </div>

      {estimates.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No estimates" description="Submit a workload YAML to see cost estimates" />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Provider</th>
                  <th className="text-right text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">CPU/mo</th>
                  <th className="text-right text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Mem/mo</th>
                  <th className="text-right text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Storage/mo</th>
                  <th className="text-right text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Total/mo</th>
                  <th className="text-right text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Hourly</th>
                </tr>
              </thead>
              <tbody>
                {estimates.map((est) => (
                  <tr
                    key={est.provider}
                    className={`border-b border-zinc-800/50 transition-colors ${
                      est.provider === cheapest
                        ? 'bg-emerald-500/5 hover:bg-emerald-500/10'
                        : 'hover:bg-zinc-800/30'
                    }`}
                  >
                    <td className="py-3 px-4">
                      <span className="font-medium text-zinc-200">{est.provider}</span>
                      {est.provider === cheapest && (
                        <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                          Cheapest
                        </span>
                      )}
                    </td>
                    <td className="py-3 px-4 text-right text-sm text-zinc-300">{formatUSD(est.cpu_cost_monthly)}</td>
                    <td className="py-3 px-4 text-right text-sm text-zinc-300">{formatUSD(est.memory_cost_monthly)}</td>
                    <td className="py-3 px-4 text-right text-sm text-zinc-300">{formatUSD(est.storage_cost_monthly)}</td>
                    <td className={`py-3 px-4 text-right text-sm font-medium ${
                      est.provider === cheapest ? 'text-emerald-400' : 'text-zinc-200'
                    }`}>
                      {formatUSD(est.total_monthly)}
                    </td>
                    <td className="py-3 px-4 text-right text-sm text-zinc-400">{formatUSD(est.total_hourly)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
