// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState } from 'react';
import { Bot, Plus, Trash2, Zap } from 'lucide-react';
import { apiDelete, apiFetchSettled, apiPost } from '../../utils/api';
import { useApiPage } from '../../hooks/useApiPage';
import PageFrame from '../PageFrame';
import PageTabs from '../PageTabs';
import EmptyState from '../EmptyState';

type ProviderKind =
  | 'openai'
  | 'anthropic'
  | 'gemini'
  | 'xai'
  | 'azure'
  | 'ollama'
  | 'vllm'
  | 'openai_compatible'
  | 'deepseek'
  | 'mistral'
  | 'qwen'
  | 'llama';

interface ZeusProviderConfig {
  id: string;
  kind: ProviderKind;
  display_name: string;
  base_url?: string;
  organization_id?: string;
  deployment_name?: string;
  default_model: string;
  enabled: boolean;
  priority: number;
  api_key_configured?: boolean;
}

interface ZeusProviderRegistry {
  default_provider_id?: string;
  air_gapped: boolean;
  providers: ZeusProviderConfig[];
}

const KIND_OPTIONS: { value: ProviderKind; label: string }[] = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic Claude' },
  { value: 'gemini', label: 'Google Gemini' },
  { value: 'xai', label: 'xAI Grok' },
  { value: 'azure', label: 'Azure OpenAI' },
  { value: 'ollama', label: 'Ollama' },
  { value: 'vllm', label: 'vLLM' },
  { value: 'openai_compatible', label: 'OpenAI Compatible' },
  { value: 'deepseek', label: 'DeepSeek' },
  { value: 'mistral', label: 'Mistral' },
  { value: 'qwen', label: 'Qwen' },
  { value: 'llama', label: 'Meta Llama' },
];

const EMPTY: ZeusProviderConfig = {
  id: '',
  kind: 'openai',
  display_name: '',
  default_model: 'gpt-4o-mini',
  enabled: true,
  priority: 10,
};

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function AiProvidersPage() {
  const [draft, setDraft] = useState<ZeusProviderConfig>({ ...EMPTY, id: `provider-${Date.now()}` });
  const [apiKey, setApiKey] = useState('');
  const [tab, setTab] = useState<'providers' | 'local'>('providers');

  const {
    data: registry,
    loading,
    error,
    reload,
  } = useApiPage<ZeusProviderRegistry>(
    () => apiFetchSettled<ZeusProviderRegistry>('/zeus/providers'),
    [],
  );

  const saveProvider = async () => {
    if (!draft.display_name.trim()) {
      toast('Display name is required', 'error');
      return;
    }
    const res = await apiPost<ZeusProviderConfig>('/zeus/providers', {
      ...draft,
      api_key: apiKey || undefined,
    });
    if (res.success) {
      toast('Provider saved', 'success');
      setApiKey('');
      void reload();
    } else {
      toast(res.error ?? 'Save failed', 'error');
    }
  };

  const testProvider = async (id: string) => {
    const res = await apiPost<{ ok: boolean; message: string }>(`/zeus/providers/${id}/test`, {});
    toast(res.data?.message ?? res.error ?? 'Test complete', res.data?.ok ? 'success' : 'error');
  };

  const removeProvider = async (id: string) => {
    const res = await apiDelete(`/zeus/providers/${id}`);
    if (res.success) {
      toast('Provider removed', 'success');
      void reload();
    }
  };

  const toggleAirGapped = async (enabled: boolean) => {
    if (!registry) return;
    const next = { ...registry, air_gapped: enabled };
    const res = await apiPost<ZeusProviderRegistry>('/zeus/providers/registry', next);
    if (res.success && res.data) void reload();
  };

  return (
    <PageFrame
      loading={loading}
      error={error}
      hasData={registry !== null}
      onRetry={() => void reload()}
      errorTitle="AI providers unavailable"
    >
      <section className="overview-section-shell mb-6 space-y-6 p-6 sm:p-8" data-testid="ai-providers-page">
        <div>
          <h2 className="text-xl font-semibold text-white flex items-center gap-2">
            <Bot className="h-5 w-5 text-aether" />
            AI Providers
          </h2>
          <p className="mt-1 text-sm text-slate-400">
            Configure multi-LLM providers for Zeus — OpenAI, Claude, Gemini, Grok, Ollama, vLLM, and custom endpoints.
          </p>
        </div>

        <PageTabs
          tabs={[
            { id: 'providers', label: 'Providers' },
            { id: 'local', label: 'Local / Air-gapped' },
          ]}
          active={tab}
          onChange={(id) => setTab(id as typeof tab)}
        />

        {tab === 'local' ? (
          <div className="space-y-4 rounded-xl border border-white/10 p-4">
            <label className="flex items-center gap-3 text-sm text-slate-200">
              <input
                type="checkbox"
                checked={registry?.air_gapped ?? false}
                onChange={(e) => void toggleAirGapped(e.target.checked)}
                data-testid="zeus-air-gapped-toggle"
              />
              Air-gapped mode — only local providers (Ollama, vLLM, OpenAI-compatible on-prem)
            </label>
            <p className="text-xs text-slate-500">
              Supports Ollama, vLLM, LM Studio, Open WebUI, and Hugging Face TGI via OpenAI-compatible endpoints.
            </p>
          </div>
        ) : null}

        <div className="grid gap-4 lg:grid-cols-2">
          <div className="space-y-3 rounded-xl border border-white/10 p-4">
            <h3 className="text-sm font-medium text-slate-200">Add provider</h3>
            <input
              className="input-field w-full"
              placeholder="Display name"
              aria-label="Provider display name"
              value={draft.display_name}
              onChange={(e) => setDraft({ ...draft, display_name: e.target.value })}
              data-testid="zeus-provider-name"
            />
            <select
              className="input-field w-full"
              aria-label="Provider kind"
              value={draft.kind}
              onChange={(e) => setDraft({ ...draft, kind: e.target.value as ProviderKind })}
            >
              {KIND_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
            <input
              className="input-field w-full"
              placeholder="API key (encrypted at rest)"
              aria-label="API key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              data-testid="zeus-provider-api-key"
            />
            <input
              className="input-field w-full"
              placeholder="Endpoint URL (optional)"
              aria-label="Endpoint URL"
              value={draft.base_url ?? ''}
              onChange={(e) => setDraft({ ...draft, base_url: e.target.value || undefined })}
            />
            <input
              className="input-field w-full"
              placeholder="Organization ID (optional)"
              aria-label="Organization ID"
              value={draft.organization_id ?? ''}
              onChange={(e) => setDraft({ ...draft, organization_id: e.target.value || undefined })}
            />
            <input
              className="input-field w-full"
              placeholder="Deployment name (Azure)"
              aria-label="Deployment name"
              value={draft.deployment_name ?? ''}
              onChange={(e) => setDraft({ ...draft, deployment_name: e.target.value || undefined })}
            />
            <input
              className="input-field w-full"
              placeholder="Model name"
              aria-label="Model name"
              value={draft.default_model}
              onChange={(e) => setDraft({ ...draft, default_model: e.target.value })}
              data-testid="zeus-provider-model"
            />
            <button type="button" className="btn-primary inline-flex items-center gap-2" onClick={() => void saveProvider()}>
              <Plus className="h-4 w-4" />
              Save provider
            </button>
          </div>

          <div className="space-y-2">
            {(registry?.providers ?? []).map((p) => (
              <div key={p.id} className="flex items-center justify-between rounded-xl border border-white/10 p-3">
                <div>
                  <p className="text-sm font-medium text-white">{p.display_name}</p>
                  <p className="text-xs text-slate-500">
                    {p.kind} · {p.default_model}
                    {p.api_key_configured ? ' · key configured' : ''}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    type="button"
                    className="btn-secondary text-xs"
                    aria-label={`Test ${p.display_name}`}
                    onClick={() => void testProvider(p.id)}
                  >
                    <Zap className="h-3 w-3" />
                  </button>
                  <button
                    type="button"
                    className="btn-secondary text-xs text-red-300"
                    aria-label={`Remove ${p.display_name}`}
                    onClick={() => void removeProvider(p.id)}
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                </div>
              </div>
            ))}
            {(registry?.providers.length ?? 0) === 0 ? (
              <EmptyState
                icon={<Bot size={40} />}
                title="No providers configured"
                description="Env vars are used as bootstrap defaults until you add a provider."
              />
            ) : null}
          </div>
        </div>
      </section>
    </PageFrame>
  );
}
