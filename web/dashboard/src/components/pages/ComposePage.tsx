// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState } from 'react';
import { Rocket } from 'lucide-react';
import { apiPost, apiPostRaw } from '../../utils/api';
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

export default function ComposePage() {
  const { canMutate } = useAuth();
  const [composeYaml, setComposeYaml] = useState(COMPOSE_EXAMPLE);
  const [result, setResult] = useState<ComposeValidationResult | null>(null);
  const [validateLoading, setValidateLoading] = useState(false);
  const [deployLoading, setDeployLoading] = useState(false);
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

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-slate-100">Compose import</h2>
        <p className="text-sm text-slate-500 mt-1">
          Validate dependency order and deploy a stack. Use <code className="text-aether/90">spec_yaml</code> for inline
          workloads or <code className="text-aether/90">spec</code> for file paths on the server.
        </p>
      </div>

      <YamlInput
        initialValue={composeYaml}
        resetValue={COMPOSE_EXAMPLE}
        layout="editor"
        buttonText="Validate compose"
        onSubmit={handleValidate}
        loading={validateLoading}
        showValidateButton={false}
        placeholder="Paste aether-compose.yaml..."
        footer={
          <>
            {result ? (
              <div className="mt-4 space-y-4">
                <div className="flex items-center gap-3">
                  <Badge text={result.valid ? 'VALID' : 'INVALID'} variant={result.valid ? 'green' : 'red'} />
                  <span className="text-sm text-slate-400">{result.workload_count ?? 0} workloads</span>
                </div>
                {result.deploy_order && result.deploy_order.length > 0 ? (
                  <div>
                    <h4 className="text-xs uppercase tracking-wider text-slate-500 mb-2">Deploy order</h4>
                    <ol className="space-y-1 text-sm text-slate-300">
                      {result.deploy_order.map((item, index) => (
                        <li key={`${item}-${index}`} className="flex items-center gap-2">
                          <span className="text-aether font-mono text-xs">{index + 1}.</span>
                          {item}
                        </li>
                      ))}
                    </ol>
                  </div>
                ) : null}
                {canMutate && result.valid ? (
                  <button
                    type="button"
                    onClick={() => void handleDeployStack()}
                    disabled={deployLoading}
                    className="inline-flex items-center gap-2 rounded-xl bg-aether px-4 py-2.5 text-sm font-medium text-white hover:bg-aether/90 disabled:opacity-50"
                  >
                    <Rocket className="w-4 h-4" />
                    {deployLoading ? 'Deploying stack…' : 'Deploy stack'}
                  </button>
                ) : null}
                <ValidateResultPanel policy={policyResult} />
                {deployResult ? (
                  <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-3 text-sm text-emerald-300">
                    Deployed {deployResult.count} workload(s): {deployResult.deployed.join(', ')}
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
  );
}
