// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState, useCallback } from 'react';
import { Link, useNavigate } from 'react-router';
import { AlertTriangle, AlertCircle, ShieldCheck, RefreshCw, WifiOff } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { apiFetchSettled, apiPost } from '../../utils/api';
import SpecWorkbench from '../SpecWorkbench';
import Badge, { SeverityBadge } from '../Badge';
import type { OpaEvaluation, PolicyResult } from '../../types/api';

export default function PolicyPage() {
  const navigate = useNavigate();
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
  const [result, setResult] = useState<PolicyResult | null>(null);
  const [opaResult, setOpaResult] = useState<OpaEvaluation | null>(null);
  const [opaManifest, setOpaManifest] = useState('{\n  "apiVersion": "v1",\n  "kind": "ConfigMap",\n  "metadata": { "name": "example", "labels": { "owner": "team-a" } }\n}');
  const [opaConfigured, setOpaConfigured] = useState(false);
  const [opaProbeFailed, setOpaProbeFailed] = useState(false);
  const [opaProbeLoading, setOpaProbeLoading] = useState(true);
  const [loading, setLoading] = useState(false);
  const [opaLoading, setOpaLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [opaError, setOpaError] = useState<string | null>(null);

  const loadOpaStatus = useCallback(async () => {
    setOpaProbeLoading(true);
    setOpaProbeFailed(false);
    const result = await apiFetchSettled<{ opa?: { configured?: boolean } }>('/server');
    if (!result.ok) {
      setOpaProbeFailed(true);
      setOpaConfigured(false);
    } else {
      setOpaConfigured(Boolean(result.data?.opa?.configured));
    }
    setOpaProbeLoading(false);
  }, []);

  useEffect(() => {
    void loadOpaStatus();
  }, [loadOpaStatus]);

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

  const policyResultPanel = result ? (
    <div className="space-y-4" data-testid="policy-check-result">
      <div className="flex items-center gap-3">
        <Badge text={result.passed ? 'PASSED' : 'FAILED'} variant={result.passed ? 'green' : 'red'} />
        <span className="text-sm text-slate-400">{result.policies_evaluated} policies evaluated</span>
      </div>
      {result.violations.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-slate-300 mb-2 flex items-center gap-2">
            <AlertCircle size={16} className="text-red-400" />
            Violations ({result.violations.length})
          </h4>
          <div className="space-y-2">
            {result.violations.map((v, i) => (
              <div key={i} className="bg-red-500/5 border border-red-500/20 rounded-lg p-3">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-medium text-slate-200 text-sm">{v.policy}</span>
                  <SeverityBadge severity={v.severity} />
                </div>
                <div className="text-sm text-slate-300">{v.message}</div>
                <div className="flex gap-4 text-xs text-slate-500 mt-1">
                  <span>Rule: {v.rule}</span>
                  <span>Field: {v.field}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      {result.warnings.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-slate-300 mb-2 flex items-center gap-2">
            <AlertTriangle size={16} className="text-amber-400" />
            Warnings ({result.warnings.length})
          </h4>
          <div className="space-y-2">
            {result.warnings.map((w, i) => (
              <div key={i} className="bg-amber-500/5 border border-amber-500/20 rounded-lg p-3">
                <div className="font-medium text-slate-200 text-sm mb-1">{w.policy}</div>
                <div className="text-sm text-slate-300">{w.message}</div>
                <div className="text-xs text-amber-400 mt-1">{w.suggestion}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  ) : error ? (
    <p className="text-sm text-red-400">{error}</p>
  ) : undefined;

  return (
    <div>
      {focusedWorkload ? (
        <WorkloadContextBanner
          testId="policy-workload-context"
          workload={focusedWorkload}
          description="Policy context"
        >
          <WorkloadScopedCrossLinks
            workload={focusedWorkload}
            prefix="policy"
            showDrift
            showGitops
            showMetrics
          />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="policy-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="policy-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="policy-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('rbac'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="policy-context-rbac-link"
          >
            RBAC →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('audit'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="policy-context-audit-link"
          >
            Audit →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: focusedWorkload })}
            className="text-aether hover:underline"
            data-testid="policy-context-compose-link"
          >
            Compose →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      <div className="mb-4 flex flex-wrap gap-3">
        <button
          type="button"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('drift'), { workload: focusedWorkload })
                : viewToPath('drift'),
            )
          }
          className="text-xs text-aether hover:underline"
          data-testid="policy-context-drift-link"
        >
          Drift detection →
        </button>
        <button
          type="button"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('workloads'), { validate: '1', workload: focusedWorkload })
                : pathWithQuery(viewToPath('workloads'), { validate: '1' }),
            )
          }
          className="text-xs text-aether hover:underline"
          data-testid="policy-validate-link"
        >
          Validate workloads →
        </button>
        <button
          type="button"
          onClick={() =>
            navigate(
              focusedWorkload
                ? pathWithQuery(viewToPath('platform'), { workload: focusedWorkload })
                : viewToPath('platform'),
            )
          }
          className="text-xs text-aether hover:underline"
          data-testid="policy-platform-link"
        >
          Platform &amp; HA →
        </button>
      </div>
      {!opaProbeLoading && !opaProbeFailed && !opaConfigured ? (
        <div
          data-testid="policy-opa-setup-banner"
          className="mb-6 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-slate-800/60/80 bg-[#11151C]/80/60 px-4 py-3 text-sm text-slate-300"
        >
          <span>OPA is not configured — built-in policy check works below; enable OPA admission on Platform &amp; HA.</span>
          <button
            type="button"
            onClick={() => navigate(viewToPath('platform'))}
            className="rounded-lg border border-aether/40 bg-aether/10 px-3 py-1 text-xs font-medium text-aether hover:bg-aether/20"
          >
            Open Platform
          </button>
        </div>
      ) : null}

      {opaProbeFailed ? (
        <div
          role="status"
          className="mb-6 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-200"
        >
          <div className="flex items-center gap-2 min-w-0">
            <WifiOff className="h-4 w-4 shrink-0 text-amber-400" aria-hidden />
            <span>Could not load OPA status from the API. Built-in policy check still works below.</span>
          </div>
          <button
            type="button"
            onClick={() => void loadOpaStatus()}
            disabled={opaProbeLoading}
            className="inline-flex items-center gap-1.5 rounded-lg border border-amber-500/40 px-3 py-1 text-xs font-medium text-amber-100 hover:bg-amber-500/15 transition-colors shrink-0 disabled:opacity-50"
          >
            <RefreshCw className={`h-3.5 w-3.5${opaProbeLoading ? ' animate-spin' : ''}`} />
            Retry
          </button>
        </div>
      ) : null}

      {opaConfigured && (
        <div className="glass-panel-card mb-6">
          <h2 className="text-lg font-semibold text-slate-100 mb-2 flex items-center gap-2">
            <ShieldCheck size={20} className="text-aether" />
            OPA admission (live)
          </h2>
          <p className="text-sm text-slate-500 mb-4">
            Evaluates Kubernetes manifest JSON against the OPA server configured via{' '}
            <code className="text-slate-400">AETHER_OPA_URL</code>. With{' '}
            <code className="text-slate-400">AETHER_OPA_ENFORCE=1</code>, cluster apply is blocked on deny.
          </p>
          <textarea
            value={opaManifest}
            onChange={(e) => setOpaManifest(e.target.value)}
            rows={12}
            className={`w-full font-mono text-sm rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2 text-slate-200`}
          />
          <button
            type="button"
            data-testid="policy-opa-check-button"
            onClick={() => void handleOpaCheck()}
            disabled={opaLoading}
            className="mt-3 btn-primary disabled:opacity-50"
          >
            {opaLoading ? 'Checking…' : 'Check with OPA'}
          </button>
          {opaError && <p className="mt-2 text-sm text-red-400">{opaError}</p>}
          {opaResult && (
            <div className="mt-4" data-testid="policy-opa-result">
              <Badge text={opaResult.allowed ? 'ALLOWED' : 'DENIED'} variant={opaResult.allowed ? 'green' : 'red'} />
              {opaResult.denials.length > 0 && (
                <ul className="mt-3 space-y-2 text-sm text-slate-300">
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

      <div data-testid="policy-check-form">
      <SpecWorkbench
        title="Workload policy check"
        description="Paste workload YAML to evaluate built-in policies before deploy."
        buttonText="Check Policies"
        onSubmit={handleCheck}
        loading={loading}
        placeholder="Paste workload YAML to check policies..."
        submitTestId="policy-check-submit"
        result={policyResultPanel}
      />
      </div>
    </div>
  );
}
