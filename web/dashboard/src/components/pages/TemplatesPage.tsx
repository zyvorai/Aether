import { useState, useEffect } from 'react';
import { Wand2, Inbox } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import Modal from '../Modal';
import CodeBlock from '../CodeBlock';
import EmptyState from '../EmptyState';
import type { Template } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function TemplatesPage() {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [generateResult, setGenerateResult] = useState<string | null>(null);
  const [generateName, setGenerateName] = useState<string | null>(null);
  const [generateLoading, setGenerateLoading] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      const data = await apiFetch<Template[]>('/templates');
      setTemplates(data ?? []);
      setLoading(false);
    }
    load();
  }, []);

  async function handleGenerate(name: string) {
    setGenerateLoading(name);
    const res = await apiPost<unknown>(`/templates/${name}`);
    if (res.success) {
      setGenerateResult(JSON.stringify(res.data ?? res, null, 2));
      setGenerateName(name);
      toast(`Template "${name}" generated successfully`, 'success');
    } else {
      toast(`Failed to generate template "${name}": ${res.error ?? 'unknown error'}`, 'error');
    }
    setGenerateLoading(null);
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-amber-500" />
      </div>
    );
  }

  return (
    <div>
      {templates.length === 0 ? (
        <EmptyState icon={<Inbox size={48} />} title="No templates" description="No templates are available" />
      ) : (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Description</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">CPU</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Memory</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-4">Actions</th>
                </tr>
              </thead>
              <tbody>
                {templates.map((t) => (
                  <tr key={t.name} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-4 font-medium text-zinc-200">{t.name}</td>
                    <td className="py-3 px-4 text-sm text-zinc-400">{t.description}</td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{t.default_cpu}</td>
                    <td className="py-3 px-4 text-sm text-zinc-300">{t.default_memory}</td>
                    <td className="py-3 px-4">
                      <button
                        onClick={() => handleGenerate(t.name)}
                        disabled={generateLoading === t.name}
                        className="flex items-center gap-1.5 px-3 py-1.5 bg-amber-600 hover:bg-amber-500 disabled:opacity-50 text-white rounded-lg text-xs font-medium transition-colors"
                      >
                        <Wand2 size={12} />
                        {generateLoading === t.name ? 'Generating...' : 'Generate'}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal
        isOpen={generateResult !== null}
        onClose={() => { setGenerateResult(null); setGenerateName(null); }}
        title={`Generated: ${generateName}`}
      >
        <CodeBlock title="json">{generateResult ?? ''}</CodeBlock>
      </Modal>
    </div>
  );
}
