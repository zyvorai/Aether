import { useState } from 'react';
import { Layers, ListChecks } from 'lucide-react';
import { apiPostRaw } from '../../utils/api';
import YamlInput from '../YamlInput';
import EmptyState from '../EmptyState';
import Badge from '../Badge';
import type { ComposeValidationResult } from '../../types/api';

export default function ComposePage() {
  const [result, setResult] = useState<ComposeValidationResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleValidate(yaml: string) {
    setLoading(true);
    setError(null);
    const response = await apiPostRaw<ComposeValidationResult>('/compose/validate', yaml, 'application/yaml');
    if (response.success && response.data) {
      setResult(response.data);
    } else {
      setResult(null);
      setError(response.error ?? 'Compose validation failed');
    }
    setLoading(false);
  }

  return (
    <div>
      <div className="rounded-2xl surface-panel p-6 mb-6">
        <div className="flex items-center gap-3 mb-4">
          <Layers className="w-5 h-5 text-aether" />
          <h2 className="text-lg font-semibold text-slate-100">Compose Spec Validation</h2>
        </div>
        <YamlInput
          buttonText="Validate Compose"
          onSubmit={handleValidate}
          loading={loading}
          placeholder="Paste a compose spec to validate workload graph and deploy order..."
        />
        {error && <p className="mt-3 text-sm text-red-400">{error}</p>}
      </div>

      {!result ? (
        <EmptyState icon={<ListChecks size={48} />} title="No compose spec validated" description="Validate a compose document to inspect workload count and deploy order." />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-[0.85fr_1.15fr] gap-6">
          <div className="rounded-2xl surface-panel p-6">
            <div className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-3">Validation</div>
            <div className="flex items-center gap-3">
              <Badge text={result.valid ? 'VALID' : 'INVALID'} variant={result.valid ? 'green' : 'red'} />
              <span className="text-sm text-slate-400">
                {result.workload_count ?? 0} workloads detected
              </span>
            </div>
          </div>

          <div className="rounded-2xl surface-panel p-6">
            <div className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-4">Resolved Deploy Order</div>
            {result.deploy_order && result.deploy_order.length > 0 ? (
              <div className="space-y-3">
                {result.deploy_order.map((item, index) => (
                  <div key={`${item}-${index}`} className="flex items-center gap-3 rounded-xl bg-slate-950/60 border border-slate-800 px-4 py-3">
                    <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-aether/10 text-xs font-semibold text-aether">
                      {index + 1}
                    </div>
                    <div className="text-sm text-slate-200">{item}</div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-sm text-slate-500">No deploy order returned.</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
