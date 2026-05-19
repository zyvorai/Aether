import { useState } from 'react';
import { Save, FileText, Eye, CheckCircle } from 'lucide-react';
import { apiPost } from '../../utils/api';
import Badge from '../Badge';
import type { ValidateResponse } from '../../types/api';

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
}

const defaultForm: EditorForm = {
  name: 'my-app',
  image: 'nginx:latest',
  runtime: 'kubernetes',
  replicas: 2,
  cpu: '500m',
  memory: '512Mi',
  intent: 'balanced',
  env: 'ENV=production\nLOG_LEVEL=info',
  healthCheck: true,
};

export default function EditorPage() {
  const [form, setForm] = useState<EditorForm>(defaultForm);
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [validateResult, setValidateResult] = useState<ValidateResponse | null>(null);
  const [showPreview, setShowPreview] = useState(true);

  const runtimes = ['podman', 'docker', 'kubernetes', 'kubevirt', 'metal3'];
  const intents = ['low-latency', 'high-throughput', 'cost-optimized', 'balanced'];

  const handleChange = (field: keyof EditorForm, value: string | number | boolean) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const generateYaml = (): string => {
    const envLines = form.env
      .trim()
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        const [key, ...rest] = line.split('=');
        return `    - name: ${key}\n      value: "${rest.join('=') || ''}"`;
      })
      .join('\n');

    return `name: ${form.name}
image: ${form.image}
runtime: ${form.runtime}
replicas: ${form.replicas}
resources:
  cpu: ${form.cpu}
  memory: ${form.memory}
intent: ${form.intent}
${form.env.trim() ? `env:\n${envLines}` : ''}
${form.healthCheck ? `healthCheck:\n  httpGet:\n    path: /health\n    port: 80` : ''}`;
  };

  const handleValidate = async () => {
    setValidating(true);
    setValidateResult(null);
    const yaml = generateYaml();
    const res = await apiPost<ValidateResponse>('/validate', { yaml });
    if (res.success && res.data) {
      setValidateResult(res.data);
    } else {
      setValidateResult({ valid: false, workload_name: null, errors: [res.error ?? 'Validation failed'] });
    }
    setValidating(false);
  };

  const handleSave = async () => {
    setSaving(true);
    setResult(null);
    const res = await apiPost('/workloads', { spec_yaml: generateYaml() });
    if (res.success) {
      setResult('Workload deployed successfully.');
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
            <div className="rounded-xl border border-slate-700 bg-slate-950/60 p-3 space-y-2">
              <div className="flex items-center gap-2">
                <Badge text={validateResult.valid ? 'VALID' : 'INVALID'} variant={validateResult.valid ? 'green' : 'red'} />
                {validateResult.workload_name && (
                  <span className="text-sm text-slate-400">{validateResult.workload_name}</span>
                )}
              </div>
              {validateResult.errors.length > 0 && (
                <ul className="text-sm text-red-400 space-y-1">
                  {validateResult.errors.map((err, i) => (
                    <li key={i}>• {err}</li>
                  ))}
                </ul>
              )}
            </div>
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
            <button
              type="button"
              onClick={() => void handleSave()}
              disabled={saving}
              className="flex items-center gap-2 px-5 py-2.5 bg-aether hover:bg-aether/90 text-white rounded-xl text-sm font-medium disabled:opacity-50"
            >
              <Save className="w-4 h-4" />
              {saving ? 'Deploying…' : 'Deploy workload'}
            </button>
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
