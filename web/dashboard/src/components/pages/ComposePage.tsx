import { useState } from 'react';
import { apiPostRaw } from '../../utils/api';
import SpecWorkbench from '../SpecWorkbench';
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

  const resultPanel = result ? (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <Badge text={result.valid ? 'VALID' : 'INVALID'} variant={result.valid ? 'green' : 'red'} />
        <span className="text-sm text-slate-400">{result.workload_count ?? 0} workloads detected</span>
      </div>
      <div>
        <h4 className="text-xs uppercase tracking-wider text-slate-500 mb-3">Deploy order</h4>
        {result.deploy_order && result.deploy_order.length > 0 ? (
          <div className="space-y-2">
            {result.deploy_order.map((item, index) => (
              <div key={`${item}-${index}`} className="flex items-center gap-3 rounded-xl bg-slate-950/60 border border-slate-800 px-4 py-3">
                <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-aether/10 text-xs font-semibold text-aether">
                  {index + 1}
                </span>
                <span className="text-sm text-slate-200">{item}</span>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-sm text-slate-500">No deploy order returned.</p>
        )}
      </div>
    </div>
  ) : error ? (
    <p className="text-sm text-red-400">{error}</p>
  ) : undefined;

  return (
    <SpecWorkbench
      title="Compose spec validation"
      description="Validate a compose document to inspect workload count and deploy order."
      buttonText="Validate Compose"
      onSubmit={handleValidate}
      loading={loading}
      placeholder="Paste a compose spec to validate workload graph and deploy order..."
      result={resultPanel}
    />
  );
}
