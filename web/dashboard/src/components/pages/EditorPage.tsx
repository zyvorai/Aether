// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useState, useEffect } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
import { useNavigate, Link } from 'react-router';
import { Save, Eye, CheckCircle, Pencil } from 'lucide-react';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { markSpecValidated, markFirstDeploy } from '../../utils/onboardingState';
import { useAuth } from '../../contexts/AuthContext';
import { buildEditorWorkloadYaml } from '../../utils/workloadYaml';
import { countYamlLines, yamlEditorHeightPx } from '../../utils/editorHeight';
import ConfidentialFormFields, {
  defaultConfidentialFormState,
  type ConfidentialFormState,
} from '../ConfidentialFormFields';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import ValidateResultPanel from '../ValidateResultPanel';
import YamlCodeEditor from '../YamlCodeEditor';
import PageLoading from '../PageLoading';
import type { ValidateResponse, PolicyResult, WorkloadResponse } from '../../types/api';

interface EditorForm extends ConfidentialFormState {
  name: string;
  image: string;
  runtime: string;
  replicas: number;
  cpu: string;
  memory: string;
  intent: string;
  env: string;
  healthCheck: boolean;
  k8sNamespace: string;
  k8sServiceAccount: string;
  k8sNodeSelector: string;
  scalingEnabled: boolean;
  scalingMin: number;
  scalingMax: number;
  ingressEnabled: boolean;
  ingressHost: string;
  ingressPath: string;
  networkDenyAllIngress: boolean;
  k8sWorkloadKind: string;
  k8sGatewayEnabled: boolean;
  k8sGatewayName: string;
  k8sGatewayHost: string;
  k8sGatewayProvision: boolean;
  k8sVpaEnabled: boolean;
  k8sKedaEnabled: boolean;
  k8sCertManagerEnabled: boolean;
  k8sPdbMinAvailable: string;
  showAdvancedK8s: boolean;
}

const defaultForm: EditorForm = {
  name: 'httpd',
  image: 'httpd:latest',
  runtime: 'kubernetes',
  replicas: 2,
  cpu: '500m',
  memory: '512Mi',
  intent: 'balanced',
  env: 'ENV=production\nLOG_LEVEL=info',
  healthCheck: true,
  k8sNamespace: 'default',
  k8sServiceAccount: '',
  k8sNodeSelector: '',
  scalingEnabled: true,
  scalingMin: 2,
  scalingMax: 5,
  ingressEnabled: false,
  ingressHost: '',
  ingressPath: '/',
  networkDenyAllIngress: false,
  k8sWorkloadKind: 'deployment',
  k8sGatewayEnabled: false,
  k8sGatewayName: '',
  k8sGatewayHost: '',
  k8sGatewayProvision: false,
  k8sVpaEnabled: false,
  k8sKedaEnabled: false,
  k8sCertManagerEnabled: false,
  k8sPdbMinAvailable: '',
  showAdvancedK8s: false,
  ...defaultConfidentialFormState,
};

function EditorPage() {
  const navigate = useNavigate();
  const { canMutate } = useAuth();
  const [workloadQuery] = useQueryParam('workload', '');
  const [form, setForm] = useState<EditorForm>(defaultForm);
  const [saving, setSaving] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [validating, setValidating] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [validateResult, setValidateResult] = useState<ValidateResponse | null>(null);
  const [policyResult, setPolicyResult] = useState<PolicyResult | null>(null);
  const [showPreview, setShowPreview] = useState(true);
  const [yamlEditMode, setYamlEditMode] = useState(false);
  const [yamlDraft, setYamlDraft] = useState('');
  const [preloadLoading, setPreloadLoading] = useState(false);

  useEffect(() => {
    if (!workloadQuery.trim()) return;
    setPreloadLoading(true);
    void apiFetchSettled<WorkloadResponse[]>('/workloads').then((res) => {
      if (res.ok) {
        const match = res.data.find((w) => w.name === workloadQuery.trim());
        if (match) {
          setForm((prev) => ({
            ...prev,
            name: match.name,
            image: match.image || prev.image,
            runtime: match.runtime || prev.runtime,
          }));
        }
      }
      setPreloadLoading(false);
    });
  }, [workloadQuery]);

  const runtimes = ['podman', 'docker', 'kubernetes', 'kata', 'kubevirt'];
  const intents = ['low-latency', 'high-throughput', 'cost-optimized', 'balanced'];

  const handleChange = (field: keyof EditorForm, value: string | number | boolean | undefined) => {
    if (value === undefined) return;
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const handleConfidentialChange = <K extends keyof ConfidentialFormState>(
    field: K,
    value: ConfidentialFormState[K],
  ) => {
    handleChange(field as keyof EditorForm, value as EditorForm[keyof EditorForm]);
  };

  const generateYaml = (): string => buildEditorWorkloadYaml(form);
  const yamlPreview = useMemo(() => buildEditorWorkloadYaml(form), [form]);
  const activeYaml = yamlEditMode && yamlDraft.trim() ? yamlDraft : yamlPreview;
  const previewLineCount = countYamlLines(activeYaml);
  const previewHeightPx = yamlEditorHeightPx(previewLineCount, false);

  function enableYamlEdit() {
    setYamlDraft(yamlPreview);
    setYamlEditMode(true);
  }

  function disableYamlEdit() {
    setYamlEditMode(false);
    setYamlDraft('');
  }

  const handleValidate = async () => {
    setValidating(true);
    setValidateResult(null);
    setPolicyResult(null);
    const yaml = activeYaml.trim();

    const [validateRes, policyRes] = await Promise.all([
      apiPost<ValidateResponse>('/validate', { yaml }),
      apiPost<PolicyResult>('/policy/check', { yaml }),
    ]);
    if (validateRes.success && validateRes.data) {
      setValidateResult(validateRes.data);
      if (validateRes.data.valid) {
        markSpecValidated();
      }
    } else {
      setValidateResult({ valid: false, workload_name: null, errors: [validateRes.error ?? 'Validation failed'] });
    }
    setPolicyResult(policyRes.data ?? null);
    setValidating(false);
  };

  const handleSave = async () => {
    if (!canMutate) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', { detail: { message: 'Read-only session — deploy is disabled', type: 'error' } }),
      );
      return;
    }
    setSaving(true);
    setResult(null);
    const yaml = activeYaml.trim();

    const validateRes = await apiPost<ValidateResponse>('/validate', { yaml });
    setValidateResult(validateRes.data ?? null);
    if (!validateRes.data?.valid) {
      setSaving(false);
      setResult('Error: Fix validation errors before deploying.');
      return;
    }

    const policyRes = await apiPost<PolicyResult>('/policy/check', { yaml });
    setPolicyResult(policyRes.data ?? null);
    if (policyRes.data && !policyRes.data.passed) {
      setSaving(false);
      setResult('Error: Policy check failed — review violations before deploying.');
      return;
    }

    const res = await apiPost('/workloads', { spec_yaml: yaml });
    if (res.success) {
      markFirstDeploy();
      setResult('Workload deployed successfully.');
      window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message: 'Workload deployed from editor', type: 'success' } }));
      navigate(pathWithQuery(viewToPath('workloads'), { workload: form.name }));
    } else {
      setResult(`Error: ${res.error ?? 'Unknown error'}`);
    }
    setSaving(false);
  };

  const handleHelmExport = async () => {
    setExporting(true);
    setExportPath(null);
    const yaml = generateYaml();
    const res = await apiPost<{ output_dir?: string }>('/helm/export', { yaml });
    setExporting(false);
    if (res.success && res.data?.output_dir) {
      setExportPath(res.data.output_dir);
      window.dispatchEvent(
        new CustomEvent('aether-toast', { detail: { message: `Helm chart exported to ${res.data.output_dir}`, type: 'success' } }),
      );
    } else {
      window.dispatchEvent(
        new CustomEvent('aether-toast', { detail: { message: res.error ?? 'Helm export failed', type: 'error' } }),
      );
    }
  };

  if (preloadLoading) {
    return <PageLoading label="Loading workload…" variant="spinner" />;
  }

  return (
    <div>
      {workloadQuery.trim() && form.name === workloadQuery ? (
        <WorkloadContextBanner
          testId="editor-workload-context"
          workload={workloadQuery.trim()}
          description="Editor context for workload"
        >
          <WorkloadScopedCrossLinks workload={workloadQuery} prefix="editor" />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('gitops'), { workload: workloadQuery.trim() })}
            className="text-primary hover:underline"
            data-testid="editor-banner-gitops-link"
          >
            GitOps →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('backups'), { workload: workloadQuery.trim() })}
            className="text-primary hover:underline"
            data-testid="editor-banner-backups-link"
          >
            Backups →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('templates'), { workload: workloadQuery.trim() })}
            className="text-primary hover:underline"
            data-testid="editor-banner-templates-link"
          >
            Templates →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadQuery.trim() })}
            className="text-primary hover:underline"
            data-testid="editor-banner-policy-link"
          >
            Policy →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      <section>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <div>
            <Link to={viewToPath('templates')} className="text-xs text-primary hover:underline" data-testid="editor-templates-link">
              Browse templates →
            </Link>
            {workloadQuery.trim() && form.name === workloadQuery ? (
              <>
                {' · '}
                <Link
                  to={pathWithQuery(viewToPath('policy'), { workload: workloadQuery.trim() })}
                  className="text-xs text-primary hover:underline"
                  data-testid="editor-policy-link"
                >
                  Policy check →
                </Link>
                {' · '}
                <Link
                  to={pathWithQuery(viewToPath('gitops'), { workload: workloadQuery.trim() })}
                  className="text-xs text-primary hover:underline"
                  data-testid="editor-gitops-link"
                >
                  GitOps →
                </Link>
              </>
            ) : null}
          </div>
        </div>
        <button
          type="button"
          onClick={() => setShowPreview(!showPreview)}
          className="flex items-center gap-2 px-3 py-1.5 text-sm border glass-divider rounded-xl hover:bg-white/[0.04] text-muted"
        >
          <Eye className="w-4 h-4" />
          {showPreview ? 'Hide' : 'Show'} YAML
        </button>
        {showPreview ? (
          <button
            type="button"
            onClick={() => (yamlEditMode ? disableYamlEdit() : enableYamlEdit())}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border glass-divider rounded-xl hover:bg-white/[0.04] text-muted"
          >
            <Pencil className="w-4 h-4" />
            {yamlEditMode ? 'Sync from form' : 'Edit YAML directly'}
          </button>
        ) : null}
      </div>

      <div className={`grid gap-6 ${showPreview ? 'grid-cols-1 lg:grid-cols-2' : 'grid-cols-1'}`}>
        <div className="glass space-y-6">
          <div>
            <h3 className="text-sm font-medium text-muted mb-3">Basic information</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-subtle mb-1">Name</label>
                <input
                  type="text"
                  value={form.name}
                  onChange={(e) => handleChange('name', e.target.value)}
                  className="glass-input"
                />
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Image</label>
                <input
                  type="text"
                  value={form.image}
                  onChange={(e) => handleChange('image', e.target.value)}
                  className="glass-input"
                />
              </div>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs text-subtle mb-1">Runtime</label>
              <select
                value={form.runtime}
                onChange={(e) => handleChange('runtime', e.target.value)}
                className="glass-input"
              >
                {runtimes.map((r) => (
                  <option key={r} value={r}>
                    {r}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-subtle mb-1">Replicas</label>
              <input
                type="number"
                value={form.replicas}
                onChange={(e) => handleChange('replicas', parseInt(e.target.value, 10) || 1)}
                className="glass-input"
              />
            </div>
          </div>

          {(form.runtime === 'kubernetes' || form.runtime === 'kata') && (
            <div>
              <h3 className="text-sm font-medium text-muted mb-3">Kubernetes</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs text-subtle mb-1">Namespace</label>
                  <input
                    type="text"
                    value={form.k8sNamespace}
                    onChange={(e) => handleChange('k8sNamespace', e.target.value)}
                    className="glass-input"
                  />
                </div>
                <div>
                  <label className="block text-xs text-subtle mb-1">Service account</label>
                  <input
                    type="text"
                    value={form.k8sServiceAccount}
                    onChange={(e) => handleChange('k8sServiceAccount', e.target.value)}
                    className="glass-input"
                  />
                </div>
                <div className="md:col-span-2">
                  <label className="block text-xs text-subtle mb-1">Node selector (key=value)</label>
                  <input
                    type="text"
                    value={form.k8sNodeSelector}
                    onChange={(e) => handleChange('k8sNodeSelector', e.target.value)}
                    placeholder="kubernetes.io/arch=amd64"
                    className="glass-input"
                  />
                </div>
              </div>
              <label className="flex items-center gap-2 text-sm text-muted mt-3">
                <input
                  type="checkbox"
                  checked={form.networkDenyAllIngress}
                  onChange={(e) => handleChange('networkDenyAllIngress', e.target.checked)}
                  className="accent-primary"
                />
                Deny all ingress (network policy)
              </label>
              <label className="flex items-center gap-2 text-sm text-muted mt-3">
                <input
                  type="checkbox"
                  checked={form.showAdvancedK8s}
                  onChange={(e) => handleChange('showAdvancedK8s', e.target.checked)}
                  className="accent-primary"
                />
                Advanced Kubernetes options
              </label>
              {form.showAdvancedK8s && (
                <div className="mt-3 grid grid-cols-1 md:grid-cols-2 gap-4 rounded-xl border glass-divider p-4">
                  <div>
                    <label className="block text-xs text-subtle mb-1">Workload kind</label>
                    <select
                      value={form.k8sWorkloadKind}
                      onChange={(e) => handleChange('k8sWorkloadKind', e.target.value)}
                      className="glass-input"
                    >
                      <option value="deployment">Deployment</option>
                      <option value="statefulSet">StatefulSet</option>
                      <option value="daemonSet">DaemonSet</option>
                      <option value="job">Job</option>
                      <option value="cronJob">CronJob</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-xs text-subtle mb-1">PDB minAvailable</label>
                    <input
                      type="text"
                      value={form.k8sPdbMinAvailable}
                      onChange={(e) => handleChange('k8sPdbMinAvailable', e.target.value)}
                      placeholder="1"
                      className="glass-input"
                    />
                  </div>
                  <label className="flex items-center gap-2 text-sm text-muted md:col-span-2">
                    <input
                      type="checkbox"
                      checked={form.k8sVpaEnabled}
                      onChange={(e) => handleChange('k8sVpaEnabled', e.target.checked)}
                      className="accent-primary"
                    />
                    Vertical Pod Autoscaler (VPA)
                  </label>
                  <label className="flex items-center gap-2 text-sm text-muted md:col-span-2">
                    <input
                      type="checkbox"
                      checked={form.k8sKedaEnabled}
                      onChange={(e) => handleChange('k8sKedaEnabled', e.target.checked)}
                      className="accent-primary"
                    />
                    KEDA ScaledObject
                  </label>
                  <label className="flex items-center gap-2 text-sm text-muted md:col-span-2">
                    <input
                      type="checkbox"
                      checked={form.k8sCertManagerEnabled}
                      onChange={(e) => handleChange('k8sCertManagerEnabled', e.target.checked)}
                      className="accent-primary"
                    />
                    cert-manager Certificate (requires Ingress host)
                  </label>
                  <label className="flex items-center gap-2 text-sm text-muted md:col-span-2">
                    <input
                      type="checkbox"
                      checked={form.k8sGatewayEnabled}
                      onChange={(e) => handleChange('k8sGatewayEnabled', e.target.checked)}
                      className="accent-primary"
                    />
                    Gateway API HTTPRoute
                  </label>
                  {form.k8sGatewayEnabled && (
                    <>
                      <input
                        type="text"
                        value={form.k8sGatewayName}
                        onChange={(e) => handleChange('k8sGatewayName', e.target.value)}
                        placeholder="Gateway name"
                        className="glass-input"
                      />
                      <input
                        type="text"
                        value={form.k8sGatewayHost}
                        onChange={(e) => handleChange('k8sGatewayHost', e.target.value)}
                        placeholder="Gateway host"
                        className="glass-input"
                      />
                      <label className="flex items-center gap-2 text-sm text-muted md:col-span-2">
                        <input
                          type="checkbox"
                          checked={form.k8sGatewayProvision}
                          onChange={(e) => handleChange('k8sGatewayProvision', e.target.checked)}
                          className="accent-primary"
                        />
                        Provision Gateway CR
                      </label>
                    </>
                  )}
                </div>
              )}
            </div>
          )}

          <div>
            <h3 className="text-sm font-medium text-muted mb-3">Scaling & ingress</h3>
            <label className="flex items-center gap-2 text-sm text-muted mb-3">
              <input
                type="checkbox"
                checked={form.scalingEnabled}
                onChange={(e) => handleChange('scalingEnabled', e.target.checked)}
                className="accent-primary"
              />
              Enable HPA-style scaling block
            </label>
            {form.scalingEnabled && (
              <div className="grid grid-cols-2 gap-4 mb-4">
                <div>
                  <label className="block text-xs text-subtle mb-1">Min replicas</label>
                  <input
                    type="number"
                    value={form.scalingMin}
                    onChange={(e) => handleChange('scalingMin', parseInt(e.target.value, 10) || 1)}
                    className="glass-input"
                  />
                </div>
                <div>
                  <label className="block text-xs text-subtle mb-1">Max replicas</label>
                  <input
                    type="number"
                    value={form.scalingMax}
                    onChange={(e) => handleChange('scalingMax', parseInt(e.target.value, 10) || 1)}
                    className="glass-input"
                  />
                </div>
              </div>
            )}
            <label className="flex items-center gap-2 text-sm text-muted mb-3">
              <input
                type="checkbox"
                checked={form.ingressEnabled}
                onChange={(e) => handleChange('ingressEnabled', e.target.checked)}
                className="accent-primary"
              />
              Expose via Ingress
            </label>
            {form.ingressEnabled && (
              <div className="grid grid-cols-2 gap-4">
                <input
                  type="text"
                  value={form.ingressHost}
                  onChange={(e) => handleChange('ingressHost', e.target.value)}
                  placeholder="app.example.com"
                  className="glass-input"
                />
                <input
                  type="text"
                  value={form.ingressPath}
                  onChange={(e) => handleChange('ingressPath', e.target.value)}
                  placeholder="/"
                  className="glass-input"
                />
              </div>
            )}
          </div>

          <div className="rounded-xl border border-lavender/20 bg-lavender/5 p-4 space-y-3">
            <h3 className="text-sm font-medium text-lavender">Confidential computing (Ragnarok / Aether)</h3>
            <ConfidentialFormFields
              runtime={form.runtime}
              state={form}
              onChange={handleConfidentialChange}
            />
          </div>

          <div>
            <h3 className="text-sm font-medium text-muted mb-3">Resources</h3>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-subtle mb-1">CPU</label>
                <input
                  type="text"
                  value={form.cpu}
                  onChange={(e) => handleChange('cpu', e.target.value)}
                  className="glass-input"
                />
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Memory</label>
                <input
                  type="text"
                  value={form.memory}
                  onChange={(e) => handleChange('memory', e.target.value)}
                  className="glass-input"
                />
              </div>
            </div>
          </div>

          <div>
            <label className="block text-xs text-subtle mb-1">Intent</label>
            <select
              value={form.intent}
              onChange={(e) => handleChange('intent', e.target.value)}
              className="glass-input"
            >
              {intents.map((i) => (
                <option key={i} value={i}>
                  {i}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-xs text-subtle mb-1">Environment (KEY=VALUE per line)</label>
            <textarea
              value={form.env}
              onChange={(e) => handleChange('env', e.target.value)}
              rows={3}
              className="glass-input font-mono text-foreground"
            />
          </div>

          <label className="flex items-center gap-2 text-sm text-muted">
            <input
              type="checkbox"
              checked={form.healthCheck}
              onChange={(e) => handleChange('healthCheck', e.target.checked)}
              className="accent-primary"
            />
            Enable HTTP health check
          </label>

          {validateResult && (
            <ValidateResultPanel validate={validateResult} policy={policyResult} />
          )}

          {result && (
            <p className={`text-sm p-3 rounded-xl border ${result.startsWith('Error') ? 'border-danger/30 text-danger bg-danger/5' : 'border-success/30 text-success bg-success/5'}`}>
              {result}
            </p>
          )}

          {exportPath && (
            <p className="text-xs text-subtle mt-2">Helm output: <code className="text-primary/90">{exportPath}</code></p>
          )}

          <div className="flex flex-wrap gap-3 pt-4 glass-divider-t">
            <button
              type="button"
              data-testid="editor-validate-button"
              onClick={() => void handleValidate()}
              disabled={validating}
              className="flex items-center gap-2 btn-secondary disabled:opacity-50"
            >
              <CheckCircle className="w-4 h-4" />
              {validating ? 'Validating…' : 'Validate'}
            </button>
            {canMutate ? (
            <button
              type="button"
              data-testid="editor-deploy-button"
              onClick={() => void handleSave()}
              disabled={saving}
              className="flex items-center gap-2 btn-primary disabled:opacity-50"
            >
              <Save className="w-4 h-4" />
              {saving ? 'Deploying…' : 'Deploy workload'}
            </button>
            ) : null}
            <button
              type="button"
              data-testid="editor-export-helm"
              onClick={() => void handleHelmExport()}
              disabled={exporting}
              className="flex items-center gap-2 px-4 py-2.5 border glass-divider glass-inset-hover rounded-xl text-sm text-muted disabled:opacity-50"
            >
              {exporting ? 'Exporting…' : 'Export Helm chart'}
            </button>
            <button
              type="button"
              data-testid="editor-reset-button"
              onClick={() => {
                setForm(defaultForm);
                setResult(null);
                setValidateResult(null);
              }}
              className="px-5 py-2.5 border glass-divider glass-inset-hover rounded-xl text-sm text-muted"
            >
              Reset
            </button>
          </div>
        </div>

        {showPreview && (
          <div className="glass flex min-h-0 flex-col">
            <div className="mb-3 flex items-center gap-2 text-sm text-muted">
              <Eye className="h-4 w-4" /> Live YAML preview
            </div>
            <div className="min-h-0 flex-1" data-testid="editor-yaml-preview">
              <YamlCodeEditor
                value={activeYaml}
                onChange={(next) => {
                  setYamlDraft(next);
                  if (!yamlEditMode) setYamlEditMode(true);
                }}
                heightPx={previewHeightPx}
                readOnly={!yamlEditMode}
                aria-label="Workload YAML preview"
              />
            </div>
          </div>
        )}
      </div>
      </section>
    </div>
  );
}

export default withAuroraPage('editor', EditorPage);
