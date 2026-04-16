import { useState } from 'react';
import { AlertTriangle, AlertCircle, Inbox } from 'lucide-react';
import { apiPost } from '../../utils/api';
import YamlInput from '../YamlInput';
import Badge, { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import type { PolicyResult } from '../../types/api';

export default function PolicyPage() {
  const [result, setResult] = useState<PolicyResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleCheck(yaml: string) {
    setLoading(true);
    setError(null);
    const res = await apiPost<PolicyResult>('/policy/check', { yaml });
    if (res.success && res.data) {
      setResult(res.data);
    } else {
      setError(res.error ?? 'Policy check failed');
      setResult(null);
    }
    setLoading(false);
  }

  return (
    <div>
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 mb-6">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4">Workload YAML</h2>
        <YamlInput
          buttonText="Check Policies"
          onSubmit={handleCheck}
          loading={loading}
          placeholder="Paste workload YAML to check policies..."
        />
        {error && <p className="mt-2 text-sm text-red-400">{error}</p>}
      </div>

      {result === null ? (
        <EmptyState icon={<Inbox size={48} />} title="No results" description="Submit a workload YAML to check policies" />
      ) : (
        <div className="space-y-6">
          {/* Summary */}
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
            <div className="flex items-center gap-3">
              <Badge
                text={result.passed ? 'PASSED' : 'FAILED'}
                variant={result.passed ? 'green' : 'red'}
              />
              <span className="text-sm text-zinc-400">
                {result.policies_evaluated} policies evaluated
              </span>
            </div>
          </div>

          {/* Violations */}
          {result.violations.length > 0 && (
            <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
              <h2 className="text-lg font-semibold text-zinc-100 mb-4 flex items-center gap-2">
                <AlertCircle size={20} className="text-red-400" />
                Violations ({result.violations.length})
              </h2>
              <div className="space-y-3">
                {result.violations.map((v, i) => (
                  <div key={i} className="bg-red-500/5 border border-red-500/20 rounded-lg p-4">
                    <div className="flex items-center gap-2 mb-2">
                      <span className="font-medium text-zinc-200">{v.policy}</span>
                      <SeverityBadge severity={v.severity} />
                    </div>
                    <div className="text-sm text-zinc-300 mb-1">{v.message}</div>
                    <div className="flex gap-4 text-xs text-zinc-500">
                      <span>Rule: {v.rule}</span>
                      <span>Field: {v.field}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Warnings */}
          {result.warnings.length > 0 && (
            <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
              <h2 className="text-lg font-semibold text-zinc-100 mb-4 flex items-center gap-2">
                <AlertTriangle size={20} className="text-amber-400" />
                Warnings ({result.warnings.length})
              </h2>
              <div className="space-y-3">
                {result.warnings.map((w, i) => (
                  <div key={i} className="bg-amber-500/5 border border-amber-500/20 rounded-lg p-4">
                    <div className="font-medium text-zinc-200 mb-1">{w.policy}</div>
                    <div className="text-sm text-zinc-300 mb-1">{w.message}</div>
                    <div className="text-xs text-amber-400">{w.suggestion}</div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
