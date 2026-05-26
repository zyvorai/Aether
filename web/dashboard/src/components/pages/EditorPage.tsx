import { useState } from 'react';
import { useNavigate } from 'react-router';
import { Save, FileText, Eye, CheckCircle } from 'lucide-react';
import { apiPost } from '../../utils/api';
import { markSpecValidated, markFirstDeploy } from '../../utils/onboardingState';
import { useAuth } from '../../contexts/AuthContext';
import { buildEditorWorkloadYaml } from '../../utils/workloadYaml';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery } from '../../utils/urlState';
import ValidateResultPanel from '../ValidateResultPanel';
import type { ValidateResponse, PolicyResult } from '../../types/api';

interface EditorForm {
  name: string;
  image: string;
  runtime: string;
  replicas: number;
  cpu: string;
  memory: string;
  intent: string;
  env: string;
  healthCheck: boolean;
  confidentialEnabled: boolean;
  confidentialTee: 'sev-snp' | 'tdx';
  confidentialSecurityProfile: '' | 'sandbox' | 'standard-confidential' | 'sovereign-high';
  confidentialKataRuntime: 'kata-clh-snp' | 'kata-clh-tdx' | 'kata-qemu-snp' | 'kata-qemu-tdx';
  attestationRequired: boolean;
  imageDigest: string;
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
  confidentialEnabled: false,
  confidentialTee: 'sev-snp',
  confidentialSecurityProfile: 'standard-confidential',
  confidentialKataRuntime: 'kata-clh-snp',
  attestationRequired: true,
  imageDigest: '',
};

export default function EditorPage() {
  const navigate = useNavigate();
  const { canMutate } = useAuth();
  const [form, setForm] = useState<EditorForm>(defaultForm);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [validateResult, setValidateResult] = useState<ValidateResponse | null>(null);
  const [policyResult, setPolicyResult] = useState<PolicyResult | null>(null);
  const [showPreview, setShowPreview] = useState(true);

  const runtimes = ['podman', 'docker', 'kubernetes', 'kata', 'kubevirt', 'metal3'];
  const intents = ['low-latency', 'high-throughput', 'cost-optimized', 'balanced'];

  const handleChange = (field: keyof EditorForm, value: string | number | boolean) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const generateYaml = (): string => buildEditorWorkloadYaml(form);

  const handleValidate = async () => {
    setValidating(true);
    setValidateResult(null);
    setPolicyResult(null);
    const yaml = generateYaml();
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
    const yaml = generateYaml();

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

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <FileText className="w-6 h-6 text-aether" />
          <div>
            <h2 className="text-lg font-semibold text-slate-100">Visual workload editor</h2>
            <p className="text-sm text-slate-500">Design workloads without writing YAML by hand</p>
          </div>
        </div>
        <button
          type="button"
          onClick={() => setShowPreview(!showPreview)}
          className="flex items-center gap-2 px-3 py-1.5 text-sm border border-slate-700 rounded-xl hover:bg-slate-800/80 text-slate-300"
        >
          <Eye className="w-4 h-4" />
          {showPreview ? 'Hide' : 'Show'} YAML
        </button>
      </div>

      <div className={`grid gap-6 ${showPreview ? 'grid-cols-1 lg:grid-cols-2' : 'grid-cols-1'}`}>
        <div className="dash-card space-y-6">
          <div>
            <h3 className="text-sm font-medium text-slate-400 mb-3">Basic information</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-slate-500 mb-1">Name</label>
                <input
                  type="text"
                  value={form.name}
                  onChange={(e) => handleChange('name', e.target.value)}
                  className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
                />
              </div>
              <div>
                <label className="block text-xs text-slate-500 mb-1">Image</label>
                <input
                  type="text"
                  value={form.image}
                  onChange={(e) => handleChange('image', e.target.value)}
                  className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
                />
              </div>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs text-slate-500 mb-1">Runtime</label>
              <select
                value={form.runtime}
                onChange={(e) => handleChange('runtime', e.target.value)}
                className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
              >
                {runtimes.map((r) => (
                  <option key={r} value={r}>
                    {r}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-slate-500 mb-1">Replicas</label>
              <input
                type="number"
                value={form.replicas}
                onChange={(e) => handleChange('replicas', parseInt(e.target.value, 10) || 1)}
                className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
              />
            </div>
          </div>

          {(form.runtime === 'kubevirt' || form.runtime === 'kata' || form.runtime === 'kubernetes') && (
            <div className="rounded-xl border border-purple-500/20 bg-purple-500/5 p-4 space-y-3">
              <h3 className="text-sm font-medium text-purple-300">Confidential computing (Ragnarok / Aether)</h3>
              <label className="flex items-center gap-2 text-sm text-slate-300">
                <input
                  type="checkbox"
                  checked={form.confidentialEnabled}
                  onChange={(e) => handleChange('confidentialEnabled', e.target.checked)}
                  className="rounded border-slate-600"
                />
                Enable confidential workload (SEV-SNP / TDX)
              </label>
              {form.confidentialEnabled && (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pl-6">
                  <div>
                    <label className="block text-xs text-slate-500 mb-1">TEE</label>
                    <select
                      value={form.confidentialTee}
                      onChange={(e) => {
                        const tee = e.target.value as EditorForm['confidentialTee'];
                        handleChange('confidentialTee', tee);
                        handleChange(
                          'confidentialKataRuntime',
                          tee === 'tdx' ? 'kata-clh-tdx' : 'kata-clh-snp',
                        );
                      }}
                      className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
                    >
                      <option value="sev-snp">AMD SEV-SNP</option>
                      <option value="tdx">Intel TDX</option>
                    </select>
                  </div>
                  {(form.runtime === 'kata' || form.runtime === 'kubernetes') && (
                    <div>
                      <label className="block text-xs text-slate-500 mb-1">Security profile</label>
                      <select
                        value={form.confidentialSecurityProfile}
                        onChange={(e) =>
                          handleChange(
                            'confidentialSecurityProfile',
                            e.target.value as EditorForm['confidentialSecurityProfile'],
                          )
                        }
                        className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
                      >
                        <option value="">Custom (manual runtime class)</option>
                        <option value="sandbox">sandbox — no Kata TEE pool</option>
                        <option value="standard-confidential">standard-confidential</option>
                        <option value="sovereign-high">sovereign-high</option>
                      </select>
                    </div>
                  )}
                  {(form.runtime === 'kata' || form.runtime === 'kubernetes') &&
                    !form.confidentialSecurityProfile && (
                    <div>
                      <label className="block text-xs text-slate-500 mb-1">Kata runtime class</label>
                      <select
                        value={form.confidentialKataRuntime}
                        onChange={(e) =>
                          handleChange(
                            'confidentialKataRuntime',
                            e.target.value as EditorForm['confidentialKataRuntime'],
                          )
                        }
                        className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
                      >
                        <option value="kata-clh-snp">kata-clh-snp (Cloud Hypervisor)</option>
                        <option value="kata-clh-tdx">kata-clh-tdx (Cloud Hypervisor)</option>
                        <option value="kata-qemu-snp">kata-qemu-snp (legacy QEMU)</option>
                        <option value="kata-qemu-tdx">kata-qemu-tdx (legacy QEMU)</option>
                      </select>
                    </div>
                  )}
                  <label className="flex items-end gap-2 text-sm text-slate-300 pb-2">
                    <input
                      type="checkbox"
                      checked={form.attestationRequired}
                      onChange={(e) => handleChange('attestationRequired', e.target.checked)}
                      className="rounded border-slate-600"
                    />
                    Require attestation before deploy
                  </label>
                  <div className="md:col-span-2">
                    <label className="block text-xs text-slate-500 mb-1">Launch digest (from signed catalog)</label>
                    <input
                      type="text"
                      value={form.imageDigest}
                      onChange={(e) => handleChange('imageDigest', e.target.value)}
                      placeholder="sha256 launch digest — aether confidential image verify-digest"
                      className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm font-mono text-slate-100"
                    />
                  </div>
                </div>
              )}
            </div>
          )}

          <div>
            <h3 className="text-sm font-medium text-slate-400 mb-3">Resources</h3>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-slate-500 mb-1">CPU</label>
                <input
                  type="text"
                  value={form.cpu}
                  onChange={(e) => handleChange('cpu', e.target.value)}
                  className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
                />
              </div>
              <div>
                <label className="block text-xs text-slate-500 mb-1">Memory</label>
                <input
                  type="text"
                  value={form.memory}
                  onChange={(e) => handleChange('memory', e.target.value)}
                  className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
                />
              </div>
            </div>
          </div>

          <div>
            <label className="block text-xs text-slate-500 mb-1">Intent</label>
            <select
              value={form.intent}
              onChange={(e) => handleChange('intent', e.target.value)}
              className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-slate-100"
            >
              {intents.map((i) => (
                <option key={i} value={i}>
                  {i}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-xs text-slate-500 mb-1">Environment (KEY=VALUE per line)</label>
            <textarea
              value={form.env}
              onChange={(e) => handleChange('env', e.target.value)}
              rows={3}
              className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm font-mono text-slate-100"
            />
          </div>

          <label className="flex items-center gap-2 text-sm text-slate-300">
            <input
              type="checkbox"
              checked={form.healthCheck}
              onChange={(e) => handleChange('healthCheck', e.target.checked)}
              className="accent-aether"
            />
            Enable HTTP health check
          </label>

          {validateResult && (
            <ValidateResultPanel validate={validateResult} policy={policyResult} />
          )}

          {result && (
            <p className={`text-sm p-3 rounded-xl border ${result.startsWith('Error') ? 'border-red-500/30 text-red-400 bg-red-500/5' : 'border-emerald-500/30 text-emerald-400 bg-emerald-500/5'}`}>
              {result}
            </p>
          )}

          <div className="flex flex-wrap gap-3 pt-4 border-t border-slate-800">
            <button
              type="button"
              onClick={() => void handleValidate()}
              disabled={validating}
              className="flex items-center gap-2 px-4 py-2.5 border border-slate-700 hover:bg-slate-800 rounded-xl text-sm font-medium text-slate-200 disabled:opacity-50"
            >
              <CheckCircle className="w-4 h-4" />
              {validating ? 'Validating…' : 'Validate'}
            </button>
            {canMutate ? (
            <button
              type="button"
              onClick={() => void handleSave()}
              disabled={saving}
              className="flex items-center gap-2 px-5 py-2.5 bg-aether hover:bg-aether/90 text-white rounded-xl text-sm font-medium disabled:opacity-50"
            >
              <Save className="w-4 h-4" />
              {saving ? 'Deploying…' : 'Deploy workload'}
            </button>
            ) : null}
            <button
              type="button"
              onClick={() => {
                setForm(defaultForm);
                setResult(null);
                setValidateResult(null);
              }}
              className="px-5 py-2.5 border border-slate-700 hover:bg-slate-800 rounded-xl text-sm text-slate-300"
            >
              Reset
            </button>
          </div>
        </div>

        {showPreview && (
          <div className="dash-card">
            <div className="flex items-center gap-2 mb-4 text-sm text-slate-400">
              <Eye className="w-4 h-4" /> Live YAML preview
            </div>
            <pre className="bg-slate-950 border border-slate-800 p-4 rounded-xl text-xs text-emerald-300 font-mono overflow-auto max-h-[32rem] whitespace-pre-wrap">
              {generateYaml()}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
