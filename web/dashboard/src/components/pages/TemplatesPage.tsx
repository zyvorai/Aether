import { useState, useEffect, useCallback } from 'react';
import { Rocket, Wand2, Inbox, Settings2 } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import Modal from '../Modal';
import EmptyState from '../EmptyState';
import type { Template } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function specPreview(spec: Record<string, unknown> | null): string {
  if (!spec) return '';
  const name = spec.name ?? spec.workload_name;
  const image = spec.image;
  const runtime = spec.runtime;
  const lines = ['# Generated workload preview'];
  if (name) lines.push(`name: ${name}`);
  if (image) lines.push(`image: ${image}`);
  if (runtime) lines.push(`runtime: ${runtime}`);
  if (spec.replicas != null) lines.push(`replicas: ${spec.replicas}`);
  if (spec.resources && typeof spec.resources === 'object') {
    const r = spec.resources as Record<string, string>;
    lines.push('resources:');
    if (r.cpu) lines.push(`  cpu: ${r.cpu}`);
    if (r.memory) lines.push(`  memory: ${r.memory}`);
  }
  if (lines.length <= 1) return JSON.stringify(spec, null, 2);
  return lines.join('\n');
}

export default function TemplatesPage() {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [generateResult, setGenerateResult] = useState<string | null>(null);
  const [generateName, setGenerateName] = useState<string | null>(null);
  const [generatedSpec, setGeneratedSpec] = useState<Record<string, unknown> | null>(null);
  const [generateLoading, setGenerateLoading] = useState<string | null>(null);
  const [deployLoading, setDeployLoading] = useState(false);
  const [configureTemplate, setConfigureTemplate] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [params, setParams] = useState({
    workload_name: '',
    owner: '',
    project: '',
    registry: '',
    cpu: '',
    memory: '',
    port: '',
    replicas: '',
  });

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<Template[]>('/templates');
    setTemplates(data ?? []);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleGenerate(name: string, overrides?: Record<string, unknown>) {
    setGenerateLoading(name);
    const res = await apiPost<Record<string, unknown>>(`/templates/${name}`, overrides ?? {});
    if (res.success) {
      setGenerateResult(JSON.stringify(res.data ?? res, null, 2));
      setGenerateName(name);
      setGeneratedSpec(res.data ?? null);
      toast(`Template "${name}" generated successfully`, 'success');
    } else {
      toast(`Failed to generate template "${name}": ${res.error ?? 'unknown error'}`, 'error');
    }
    setGenerateLoading(null);
  }

  async function handleDeployGenerated() {
    if (!generatedSpec || deployLoading) return;
    setDeployLoading(true);
    const res = await apiPost<string>('/workloads', { spec: generatedSpec });
    setDeployLoading(false);
    if (res.success) {
      toast(`Template "${generateName}" deployed successfully`, 'success');
    } else {
      toast(`Failed to deploy template "${generateName}": ${res.error ?? 'unknown error'}`, 'error');
    }
  }

  function buildOverrides() {
    return {
      workload_name: params.workload_name.trim() || undefined,
      owner: params.owner.trim() || undefined,
      project: params.project.trim() || undefined,
      registry: params.registry.trim() || undefined,
      cpu: params.cpu.trim() || undefined,
      memory: params.memory.trim() || undefined,
      port: params.port.trim() ? Number(params.port) : undefined,
      replicas: params.replicas.trim() ? Number(params.replicas) : undefined,
    };
  }

  const filtered = templates.filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.description.toLowerCase().includes(search.toLowerCase())
  );

  if (loading && templates.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-aether" />
      </div>
    );
  }

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search templates…"
        onRefresh={() => void load()}
        refreshing={loading}
      />

      {templates.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No templates" description="No templates are available" />
      ) : (
        <div className="dash-card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-800">
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Description</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">CPU</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Memory</th>
                  <th className="text-left text-xs uppercase tracking-wider text-slate-500 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((t) => (
                  <tr key={t.name} className="border-b border-slate-800/50 hover:bg-slate-800/30 transition-colors">
                    <td className="py-3 px-4 font-medium text-slate-200">{t.name}</td>
                    <td className="py-3 px-4 text-sm text-slate-400">{t.description}</td>
                    <td className="py-3 px-4 text-sm text-slate-300">{t.default_cpu}</td>
                    <td className="py-3 px-4 text-sm text-slate-300">{t.default_memory}</td>
                    <td className="py-3 px-4">
                      <div className="flex items-center gap-2">
                        <button
                          type="button"
                          onClick={() => void handleGenerate(t.name)}
                          disabled={generateLoading === t.name}
                          className="flex items-center gap-1.5 px-3 py-1.5 bg-aether hover:bg-aether/90 disabled:opacity-50 text-white rounded-lg text-xs font-medium transition-colors"
                        >
                          <Wand2 size={12} />
                          {generateLoading === t.name ? 'Generating…' : 'Generate'}
                        </button>
                        <button
                          type="button"
                          onClick={() => setConfigureTemplate(t.name)}
                          className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-lg text-xs font-medium transition-colors"
                        >
                          <Settings2 size={12} />
                          Configure
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {filtered.length === 0 && (
              <p className="text-sm text-slate-500 py-6 text-center">No templates match your search.</p>
            )}
          </div>
        </div>
      )}

      <Modal
        isOpen={generateResult !== null}
        onClose={() => {
          setGenerateResult(null);
          setGenerateName(null);
          setGeneratedSpec(null);
          setDeployLoading(false);
        }}
        title={`Generated: ${generateName}`}
      >
        <div className="space-y-4">
          <div className="flex items-center justify-between gap-4 rounded-xl border border-slate-700 bg-slate-950/70 px-4 py-3">
            <div>
              <div className="text-sm font-medium text-slate-100">Generated workload spec</div>
              <div className="text-xs text-slate-500">Preview and deploy the generated template.</div>
            </div>
            <button
              type="button"
              onClick={() => void handleDeployGenerated()}
              disabled={deployLoading || !generatedSpec}
              className="inline-flex items-center gap-2 rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-emerald-500 disabled:opacity-50 shrink-0"
            >
              <Rocket size={14} />
              {deployLoading ? 'Deploying…' : 'Deploy'}
            </button>
          </div>
          <div className="dash-card bg-slate-950/80 p-4">
            <h4 className="text-xs uppercase tracking-wider text-slate-500 mb-2">Preview</h4>
            <pre className="text-xs text-emerald-300 font-mono whitespace-pre-wrap overflow-auto max-h-48">
              {specPreview(generatedSpec)}
            </pre>
          </div>
          <details className="text-sm">
            <summary className="cursor-pointer text-slate-400 hover:text-slate-300">Full JSON</summary>
            <pre className="mt-2 text-xs text-slate-400 font-mono overflow-auto max-h-48">{generateResult ?? ''}</pre>
          </details>
        </div>
      </Modal>

      <Modal
        isOpen={configureTemplate !== null}
        onClose={() => setConfigureTemplate(null)}
        title={`Configure template: ${configureTemplate ?? ''}`}
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {(
            [
              ['workload_name', 'Workload name'],
              ['owner', 'Owner'],
              ['project', 'Project'],
              ['registry', 'Registry'],
              ['cpu', 'CPU'],
              ['memory', 'Memory'],
              ['port', 'Port'],
              ['replicas', 'Replicas'],
            ] as const
          ).map(([key, label]) => (
            <input
              key={key}
              value={params[key]}
              onChange={(e) => setParams((current) => ({ ...current, [key]: e.target.value }))}
              placeholder={label}
              className="rounded-xl border border-slate-700 bg-slate-950/80 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-500"
            />
          ))}
        </div>
        <div className="mt-4 flex justify-end gap-3">
          <button type="button" onClick={() => setConfigureTemplate(null)} className="px-4 py-2 rounded-lg text-sm text-slate-300 hover:bg-slate-800">
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              if (!configureTemplate) return;
              void handleGenerate(configureTemplate, buildOverrides());
              setConfigureTemplate(null);
            }}
            className="px-4 py-2 rounded-lg bg-aether hover:bg-aether/90 text-sm font-medium text-white"
          >
            Generate
          </button>
        </div>
      </Modal>
    </div>
  );
}
