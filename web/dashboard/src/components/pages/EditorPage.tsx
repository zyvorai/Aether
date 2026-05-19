import { useState } from 'react';
import { Save, FileText, Eye } from 'lucide-react';
import { apiPost } from '../../utils/api';

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
  const [result, setResult] = useState<string>('');
  const [showPreview, setShowPreview] = useState(true);

  const runtimes = ['podman', 'docker', 'kubernetes', 'kubevirt', 'metal3'];
  const intents = ['low-latency', 'high-throughput', 'cost-optimized', 'balanced'];

  const handleChange = (field: keyof EditorForm, value: any) => {
    setForm(prev => ({ ...prev, [field]: value }));
  };

  // Generate live YAML preview
  const generateYaml = (): string => {
    const envLines = form.env.trim()
      .split('\n')
      .filter(Boolean)
      .map(line => `    - name: ${line.split('=')[0]}\n      value: "${line.split('=')[1] || ''}"`)
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

  const handleSave = async () => {
    setSaving(true);
    setResult('');

    try {
      const spec = {
        name: form.name,
        image: form.image,
        runtime: form.runtime,
        replicas: form.replicas,
        resources: { cpu: form.cpu, memory: form.memory },
        intent: form.intent,
        env: form.env,
        healthCheck: form.healthCheck,
      };

      const res = await apiPost('/api/workloads', spec);
      if (res.success) {
        setResult('Workload saved and deployed successfully!');
      } else {
        setResult('Error: ' + (res.error || 'Unknown error'));
      }
    } catch (e) {
      setResult('Failed to deploy workload');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="max-w-7xl mx-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <FileText className="w-6 h-6 text-aether" />
          <div>
            <h1 className="text-2xl font-semibold">Visual Workload Editor</h1>
            <p className="text-sm text-slate-500">Design workloads without writing YAML</p>
          </div>
        </div>
        <button
          onClick={() => setShowPreview(!showPreview)}
          className="flex items-center gap-2 px-3 py-1.5 text-sm border border-slate-700 rounded-lg hover:bg-slate-800"
        >
          <Eye className="w-4 h-4" />
          {showPreview ? 'Hide' : 'Show'} YAML Preview
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Form Panel */}
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 space-y-6">
          {/* Basic */}
          <div>
            <h3 className="text-sm font-medium text-slate-400 mb-3">Basic Information</h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-slate-500 mb-1">Name</label>
                <input type="text" value={form.name} onChange={e => handleChange('name', e.target.value)} className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm" />
              </div>
              <div>
                <label className="block text-xs text-slate-500 mb-1">Image</label>
                <input type="text" value={form.image} onChange={e => handleChange('image', e.target.value)} className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm" />
              </div>
            </div>
          </div>

          {/* Runtime & Replicas */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs text-slate-500 mb-1">Runtime</label>
              <select value={form.runtime} onChange={e => handleChange('runtime', e.target.value)} className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm">
                {runtimes.map(r => <option key={r} value={r}>{r}</option>)}
              </select>
            </div>
            <div>
              <label className="block text-xs text-slate-500 mb-1">Replicas</label>
              <input type="number" value={form.replicas} onChange={e => handleChange('replicas', parseInt(e.target.value))} className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm" />
            </div>
          </div>

          {/* Resources */}
          <div>
            <h3 className="text-sm font-medium text-slate-400 mb-3">Resources</h3>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-slate-500 mb-1">CPU</label>
                <input type="text" value={form.cpu} onChange={e => handleChange('cpu', e.target.value)} className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm" />
              </div>
              <div>
                <label className="block text-xs text-slate-500 mb-1">Memory</label>
                <input type="text" value={form.memory} onChange={e => handleChange('memory', e.target.value)} className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm" />
              </div>
            </div>
          </div>

          {/* Intent */}
          <div>
            <label className="block text-xs text-slate-500 mb-1">Intent</label>
            <select value={form.intent} onChange={e => handleChange('intent', e.target.value)} className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm">
              {intents.map(i => <option key={i} value={i}>{i}</option>)}
            </select>
          </div>

          {/* Environment Variables */}
          <div>
            <label className="block text-xs text-slate-500 mb-1">Environment Variables (KEY=VALUE per line)</label>
            <textarea
              value={form.env}
              onChange={e => handleChange('env', e.target.value)}
              rows={3}
              className="w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm font-mono"
            />
          </div>

          {/* Health Check */}
          <div className="flex items-center gap-2">
            <input type="checkbox" checked={form.healthCheck} onChange={e => handleChange('healthCheck', e.target.checked)} className="accent-aether" />
            <label className="text-sm text-slate-300">Enable HTTP health check</label>
          </div>

          {/* Actions */}
          <div className="flex gap-3 pt-4 border-t border-slate-800">
            <button onClick={handleSave} disabled={saving} className="flex items-center gap-2 px-5 py-2.5 bg-aether hover:bg-aether/90 text-black rounded-lg text-sm font-medium disabled:opacity-50">
              <Save className="w-4 h-4" /> {saving ? 'Deploying...' : 'Deploy Workload'}
            </button>
            <button onClick={() => setForm(defaultForm)} className="px-5 py-2.5 border border-slate-700 hover:bg-slate-800 rounded-lg text-sm">Reset</button>
          </div>

          {result && <div className="text-sm p-3 bg-slate-950 border border-slate-700 rounded-lg text-emerald-400">{result}</div>}
        </div>

        {/* Live YAML Preview */}
        {showPreview && (
          <div className="bg-slate-950 border border-slate-800 rounded-xl p-6">
            <div className="flex items-center gap-2 mb-4 text-sm text-slate-400">
              <Eye className="w-4 h-4" /> Live YAML Preview
            </div>
            <pre className="bg-black p-4 rounded-lg text-xs text-emerald-300 font-mono overflow-auto h-[520px] whitespace-pre-wrap">
              {generateYaml()}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
