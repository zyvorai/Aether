import { useEffect, useState } from 'react';
import { AlertTriangle, AlertCircle, Inbox, ShieldCheck } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import YamlInput from '../YamlInput';
import Badge, { SeverityBadge } from '../Badge';
import EmptyState from '../EmptyState';
import type { OpaEvaluation, PolicyResult } from '../../types/api';

export default function PolicyPage() {
  const [result, setResult] = useState<PolicyResult | null>(null);
  const [opaResult, setOpaResult] = useState<OpaEvaluation | null>(null);
  const [opaManifest, setOpaManifest] = useState('{\n  "apiVersion": "v1",\n  "kind": "ConfigMap",\n  "metadata": { "name": "example", "labels": { "owner": "team-a" } }\n}');
  const [opaConfigured, setOpaConfigured] = useState(false);
  const [loading, setLoading] = useState(false);
  const [opaLoading, setOpaLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [opaError, setOpaError] = useState<string | null>(null);

  useEffect(() => {
    void apiFetch<{ opa?: { configured?: boolean } }>('/server').then((s) => {
      setOpaConfigured(Boolean(s?.opa?.configured));
    });
  }, []);

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

  async function handleOpaCheck() {
    setOpaLoading(true);
    setOpaError(null);
    try {
      const manifest = JSON.parse(opaManifest) as Record<string, unknown>;
      const res = await apiPost<OpaEvaluation>('/policy/opa', { manifest });
      if (res.success && res.data) {
        setOpaResult(res.data);
      } else {
        setOpaError(res.error ?? 'OPA check failed');
        setOpaResult(null);
      }
    } catch (e) {
      setOpaError(String(e));
      setOpaResult(null);
    }
    setOpaLoading(false);
  }

  return (
    <div>
      {opaConfigured && (
        <div className="dash-card mb-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-2 flex items-center gap-2">
            <ShieldCheck size={20} className="text-aether" />
            OPA admission (live)
          </h2>
          <p className="text-sm text-zinc-500 mb-4">
            Evaluates Kubernetes manifest JSON against the OPA server configured via <code className="text-zinc-400">AETHER_OPA_URL</code>.
            With <code className="text-zinc-400">AETHER_OPA_ENFORCE=1</code>, cluster apply is blocked on deny.
          </p>
          <textarea
            value={opaManifest}
            onChange={(e) => setOpaManifest(e.target.value)}
            rows={12}
            className="w-full font-mono text-sm rounded-xl border border-zinc-700 bg-zinc-950/80 px-3 py-2 text-zinc-200"
          />
          <button
            type="button"
            onClick={() => void handleOpaCheck()}
            disabled={opaLoading}
            className="mt-3 rounded-xl bg-aether px-4 py-2 text-sm font-medium text-white hover:bg-aether/90 disabled:opacity-50"
          >
            {opaLoading ? 'Checking…' : 'Check with OPA'}
          </button>
          {opaError && <p className="mt-2 text-sm text-red-400">{opaError}</p>}
          {opaResult && (
            <div className="mt-4">
              <Badge text={opaResult.allowed ? 'ALLOWED' : 'DENIED'} variant={opaResult.allowed ? 'green' : 'red'} />
              {opaResult.denials.length > 0 && (
                <ul className="mt-3 space-y-2 text-sm text-zinc-300">
                  {opaResult.denials.map((d) => (
                    <li key={d} className="rounded-lg bg-red-500/10 border border-red-500/20 px-3 py-2">
                      {d}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
      )}

      <div className="dash-card mb-6">
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
          <div className="dash-card">
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
            <div className="dash-card">
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
            <div className="dash-card">
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
