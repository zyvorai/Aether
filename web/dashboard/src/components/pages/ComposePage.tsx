import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState } from 'react';
import { Link } from 'react-router';
import { Rocket } from 'lucide-react';
import { apiPost, apiPostRaw } from '../../utils/api';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { useAuth } from '../../contexts/AuthContext';
import YamlInput from '../YamlInput';
import ValidateResultPanel from '../ValidateResultPanel';
import Badge from '../Badge';
import type { ComposeValidationResult, PolicyResult } from '../../types/api';

const COMPOSE_EXAMPLE = `version: "1"
workloads:
  web:
    spec_yaml: |
      apiVersion: aether/v1
      kind: Workload
      metadata:
        name: compose-web
        owner: dashboard
        project: default
      build:
        context: .
        dockerfile: Dockerfile
        registry: docker.io/library
        tag: latest
      requirements:
        cpu: 100m
        memory: 128Mi
        storage: 1Gi
      runtime:
        preferred: kube
        allow:
          - kube
      health:
        liveness:
          httpGet:
            path: /
            port: 80
          initialDelaySeconds: 10
          periodSeconds: 10
        readiness:
          httpGet:
            path: /
            port: 80
          initialDelaySeconds: 5
          periodSeconds: 5
      network:
        service: true
        ports:
          - port: 80
            protocol: TCP
`;

interface ComposeDeployResult {
  deployed: string[];
  count: number;
}

function ComposePage() {
  const { canMutate } = useAuth();
  const [workloadFocus] = useQueryParam('workload');
  const focusedWorkload = workloadFocus.trim();
  const [composeYaml, setComposeYaml] = useState(COMPOSE_EXAMPLE);
  const [result, setResult] = useState<ComposeValidationResult | null>(null);
  const [validateLoading, setValidateLoading] = useState(false);
  const [deployLoading, setDeployLoading] = useState(false);
  const [downLoading, setDownLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [policyResult, setPolicyResult] = useState<PolicyResult | null>(null);
  const [deployResult, setDeployResult] = useState<ComposeDeployResult | null>(null);

  async function handleValidate(yaml: string) {
    setValidateLoading(true);
    setError(null);
    setDeployResult(null);
    setPolicyResult(null);
    setComposeYaml(yaml);
    const response = await apiPostRaw<ComposeValidationResult>('/compose/validate', yaml, 'application/yaml');
    if (response.success && response.data) {
      setResult(response.data);
    } else {
      setResult(null);
      setError(response.error ?? 'Compose validation failed');
    }
    setValidateLoading(false);
  }

  async function handleDeployStack() {
    if (!canMutate) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', { detail: { message: 'Read-only session — deploy is disabled', type: 'error' } }),
      );
      return;
    }
    const yaml = composeYaml.trim();
    if (!yaml) return;

    setDeployLoading(true);
    setError(null);
    setDeployResult(null);
    setPolicyResult(null);

    const validateRes = await apiPostRaw<ComposeValidationResult>('/compose/validate', yaml, 'application/yaml');
    if (!validateRes.success || !validateRes.data?.valid) {
      setError(validateRes.error ?? 'Compose validation failed');
      setDeployLoading(false);
      return;
    }
    setResult(validateRes.data);

    const policyRes = await apiPost<PolicyResult>('/policy/check', { yaml });
    setPolicyResult(policyRes.data ?? null);
    if (policyRes.data && !policyRes.data.passed) {
      setError('Policy check failed — review violations before deploying');
      setDeployLoading(false);
      return;
    }

    const deployRes = await apiPostRaw<ComposeDeployResult>('/compose/up', yaml, 'application/yaml');
    setDeployLoading(false);
    if (deployRes.success && deployRes.data) {
      setDeployResult(deployRes.data);
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: { message: `Deployed ${deployRes.data.count} workload(s) from compose`, type: 'success' },
        }),
      );
    } else {
      setError(deployRes.error ?? 'Compose deploy failed');
    }
  }

  async function handleComposeDown() {
    if (!canMutate) return;
    const yaml = composeYaml.trim();
    if (!yaml) return;
    setDownLoading(true);
    setError(null);
    const res = await apiPostRaw<{ stopped?: string[]; errors?: unknown[] }>('/compose/down', yaml, 'application/yaml');
    setDownLoading(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: { message: `Stopped ${res.data?.stopped?.length ?? 0} workload(s)`, type: 'success' },
        }),
      );
    } else {
      setError(res.error ?? 'Compose down failed');
    }
  }

  return (
    <div className="space-y-6">
      {focusedWorkload ? (
        <WorkloadContextBanner
          testId="compose-workload-context"
          workload={focusedWorkload}
          description="Compose context"
        >
          <WorkloadScopedCrossLinks workload={focusedWorkload} prefix="compose" showDrift showGitops />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('deps'), { workload: focusedWorkload })}
            className="text-brand hover:underline"
            data-testid="compose-deps-link"
          >
            Dependencies →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: focusedWorkload })}
            className="text-brand hover:underline"
            data-testid="compose-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('templates'), { workload: focusedWorkload })}
            className="text-brand hover:underline"
            data-testid="compose-templates-link"
          >
            Templates →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: focusedWorkload })}
            className="text-brand hover:underline"
            data-testid="compose-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: focusedWorkload })}
            className="text-brand hover:underline"
            data-testid="compose-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('gitops'), { workload: focusedWorkload })}
            className="text-brand hover:underline"
            data-testid="compose-context-gitops-link"
          >
            GitOps →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('envs'), { workload: focusedWorkload })}
            className="text-brand hover:underline"
            data-testid="compose-context-envs-link"
          >
            Environments →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <section className="space-y-6">
      <div>
        <p className="text-sm text-subtle">
          Validate dependency order and deploy a stack. Use <code className="text-brand/90">spec_yaml</code> for inline
          workloads or <code className="text-brand/90">spec</code> for file paths on the server.
        </p>
      </div>

      <div data-testid="compose-validate-panel">
      <YamlInput
        initialValue={composeYaml}
        resetValue={COMPOSE_EXAMPLE}
        layout="editor"
        buttonText="Validate compose"
        onSubmit={handleValidate}
        loading={validateLoading}
        submitTestId="compose-validate-submit"
        showValidateButton={false}
        placeholder="Paste aether-compose.yaml..."
        footer={
          <>
            {result ? (
              <div className="mt-4 space-y-4" data-testid="compose-validate-result">
                <div className="flex items-center gap-3">
                  <Badge text={result.valid ? 'VALID' : 'INVALID'} variant={result.valid ? 'green' : 'red'} />
                  <span className="text-sm text-muted">{result.workload_count ?? 0} workloads</span>
                </div>
                {result.deploy_order && result.deploy_order.length > 0 ? (
                  <div>
                    <h4 className="text-xs uppercase tracking-wider text-subtle mb-2">Deploy order</h4>
                    <ol className="space-y-1 text-sm text-muted">
                      {result.deploy_order.map((item, index) => (
                        <li key={`${item}-${index}`} className="flex items-center gap-2">
                          <span className="text-brand font-mono text-xs">{index + 1}.</span>
                          <Link
                            to={pathWithQuery(viewToPath('workloads'), { workload: item })}
                            className="text-brand hover:underline"
                          >
                            {item}
                          </Link>
                        </li>
                      ))}
                    </ol>
                  </div>
                ) : null}
                {canMutate && result.valid ? (
                  <div className="flex flex-wrap gap-3">
                    <Link
                      to={viewToPath('deps')}
                      className="inline-flex items-center self-center text-xs text-brand hover:underline"
                      data-testid="compose-deps-link"
                    >
                      Dependency graph →
                    </Link>
                    <button
                      type="button"
                      data-testid="compose-deploy-stack"
                      onClick={() => void handleDeployStack()}
                      disabled={deployLoading}
                      className="inline-flex items-center gap-2 btn-primary disabled:opacity-50"
                    >
                      <Rocket className="w-4 h-4" />
                      {deployLoading ? 'Deploying stack…' : 'Deploy stack'}
                    </button>
                    <button
                      type="button"
                      data-testid="compose-stack-down"
                      onClick={() => void handleComposeDown()}
                      disabled={downLoading}
                      className="inline-flex items-center gap-2 rounded-xl border border-red-500/40 px-4 py-2.5 text-sm text-red-300 hover:bg-red-500/10 disabled:opacity-50"
                    >
                      {downLoading ? 'Stopping…' : 'Compose down'}
                    </button>
                  </div>
                ) : null}
                <ValidateResultPanel policy={policyResult} />
                {deployResult ? (
                  <div
                    data-testid="compose-deploy-result"
                    className="rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-3 text-sm text-emerald-300"
                  >
                    Deployed {deployResult.count} workload(s):{' '}
                    {deployResult.deployed.map((name, i) => (
                      <span key={name}>
                        {i > 0 ? ', ' : ''}
                        <Link
                          to={pathWithQuery(viewToPath('workloads'), { workload: name })}
                          className="text-emerald-200 hover:underline"
                        >
                          {name}
                        </Link>
                      </span>
                    ))}
                  </div>
                ) : null}
                {error ? <p className="text-sm text-red-400">{error}</p> : null}
              </div>
            ) : error ? (
              <p className="text-sm text-red-400 mt-3">{error}</p>
            ) : null}
          </>
        }
      />
      </div>
      </section>
    </div>
  );
}

export default withAuroraPage('compose', ComposePage);
