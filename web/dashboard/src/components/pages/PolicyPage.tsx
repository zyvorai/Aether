// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { AlertTriangle, AlertCircle, ShieldCheck } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { useApiPage } from '../../hooks/useApiPage';
import SpecWorkbench from '../SpecWorkbench';
import Badge, { SeverityBadge } from '../Badge';
import InlineActionError from '../InlineActionError';
import PanelLoadError from '../PanelLoadError';
import type { OpaEvaluation, PolicyResult } from '../../types/api';

interface ServerOpaStatus {
  opa?: { configured?: boolean };
}

function PolicyPage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
  const [result, setResult] = useState<PolicyResult | null>(null);
  const [opaResult, setOpaResult] = useState<OpaEvaluation | null>(null);
  const [opaManifest, setOpaManifest] = useState('{\n  "apiVersion": "v1",\n  "kind": "ConfigMap",\n  "metadata": { "name": "example", "labels": { "owner": "team-a" } }\n}');
  const [loading, setLoading] = useState(false);
  const [opaLoading, setOpaLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [opaError, setOpaError] = useState<string | null>(null);

  const {
    data: serverStatus,
    loading: opaProbeLoading,
    error: opaProbeFailed,
    reload: loadOpaStatus,
  } = useApiPage<ServerOpaStatus>(
    () => apiFetchSettled<ServerOpaStatus>('/server'),
    [refreshKey],
  );

  const opaConfigured = Boolean(serverStatus?.opa?.configured);

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
        <span className="text-sm text-muted">{result.policies_evaluated} policies evaluated</span>
      </div>
      {result.violations.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-muted mb-2 flex items-center gap-2">
            <AlertCircle size={16} className="text-danger" />
            Violations ({result.violations.length})
          </h4>
          <div className="space-y-2">
            {result.violations.map((v, i) => (
              <div key={i} className="bg-danger/5 border border-danger/20 rounded-lg p-3">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-medium text-foreground text-sm">{v.policy}</span>
                  <SeverityBadge severity={v.severity} />
                </div>
                <div className="text-sm text-muted">{v.message}</div>
                <div className="flex gap-4 text-xs text-subtle mt-1">
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
          <h4 className="text-sm font-medium text-muted mb-2 flex items-center gap-2">
            <AlertTriangle size={16} className="text-warning" />
            Warnings ({result.warnings.length})
          </h4>
          <div className="space-y-2">
            {result.warnings.map((w, i) => (
              <div key={i} className="bg-warning/5 border border-warning/20 rounded-lg p-3">
                <div className="font-medium text-foreground text-sm mb-1">{w.policy}</div>
                <div className="text-sm text-muted">{w.message}</div>
                <div className="text-xs text-warning mt-1">{w.suggestion}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  ) : error ? (
    <InlineActionError message={error} />
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
            className="text-primary hover:underline"
            data-testid="policy-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: focusedWorkload })}
            className="text-primary hover:underline"
            data-testid="policy-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: focusedWorkload })}
            className="text-primary hover:underline"
            data-testid="policy-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('rbac'), { workload: focusedWorkload })}
            className="text-primary hover:underline"
            data-testid="policy-context-rbac-link"
          >
            RBAC →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('audit'), { workload: focusedWorkload })}
            className="text-primary hover:underline"
            data-testid="policy-context-audit-link"
          >
            Audit →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: focusedWorkload })}
            className="text-primary hover:underline"
            data-testid="policy-context-compose-link"
          >
            Compose →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <section className="glass mb-6 space-y-6 p-6 sm:p-8">
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
          className="text-xs text-primary hover:underline"
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
          className="text-xs text-primary hover:underline"
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
          className="text-xs text-primary hover:underline"
          data-testid="policy-platform-link"
        >
          Platform &amp; HA →
        </button>
      </div>
      {!opaProbeLoading && !opaProbeFailed && !opaConfigured ? (
        <div
          data-testid="policy-opa-setup-banner"
          className="mb-6 flex flex-wrap items-center justify-between gap-3 rounded-xl border glass-divider/80 glass/60 px-4 py-3 text-sm text-muted"
        >
          <span>OPA is not configured — built-in policy check works below; enable OPA admission on Platform &amp; HA.</span>
          <button
            type="button"
            onClick={() => navigate(viewToPath('platform'))}
            className="rounded-lg border border-primary/40 bg-primary/10 px-3 py-1 text-xs font-medium text-primary hover:bg-primary/20"
          >
            Open Platform
          </button>
        </div>
      ) : null}

      {opaProbeFailed ? (
        <PanelLoadError
          title="Could not load OPA status"
          description="Built-in policy check still works below."
          onRetry={() => void loadOpaStatus()}
        />
      ) : null}

      {opaConfigured && (
        <div className="glass mb-6">
          <h2 className="text-lg font-semibold text-foreground mb-2 flex items-center gap-2">
            <ShieldCheck size={20} className="text-primary" />
            OPA admission (live)
          </h2>
          <p className="text-sm text-subtle mb-4">
            Evaluates Kubernetes manifest JSON against the OPA server configured via{' '}
            <code className="text-muted">AETHER_OPA_URL</code>. With{' '}
            <code className="text-muted">AETHER_OPA_ENFORCE=1</code>, cluster apply is blocked on deny.
          </p>
          <textarea
            value={opaManifest}
            onChange={(e) => setOpaManifest(e.target.value)}
            rows={12}
            className={`glass-input font-mono`}
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
          {opaError ? <InlineActionError message={opaError} /> : null}
          {opaResult && (
            <div className="mt-4" data-testid="policy-opa-result">
              <Badge text={opaResult.allowed ? 'ALLOWED' : 'DENIED'} variant={opaResult.allowed ? 'green' : 'red'} />
              {opaResult.denials.length > 0 && (
                <ul className="mt-3 space-y-2 text-sm text-muted">
                  {opaResult.denials.map((d) => (
                    <li key={d} className="rounded-lg bg-danger/10 border border-danger/20 px-3 py-2">
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
        buttonText={loading ? 'Checking…' : 'Check Policies'}
        onSubmit={handleCheck}
        loading={loading}
        placeholder="Paste workload YAML to check policies..."
        submitTestId="policy-check-submit"
        result={policyResultPanel}
      />
      </div>
      </section>
    </div>
  );
}

export default withAuroraPage('policy', PolicyPage);
