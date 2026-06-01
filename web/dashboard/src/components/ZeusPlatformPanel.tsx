// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Bot, Loader2, Mic, RefreshCw, Shield, Sparkles } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import GlassSection from './GlassSection';

interface ZeusMemoryReport {
  entries: Array<{
    session_id: string;
    summary: string;
    updated_at: string;
  }>;
  persisted: boolean;
}

interface LlmProviderStatus {
  active_provider: string;
  openai_configured: boolean;
  anthropic_configured: boolean;
  ollama_configured: boolean;
  model: string;
  fallback_rule_based: boolean;
}

interface VoiceLab {
  status: string;
  hint: string;
  supported: boolean;
  sample_transcript: string;
}

interface MultiAgentRoute {
  agent: string;
  label: string;
  confidence: number;
  suggested_prompts: string[];
}

interface ZeusAuditReport {
  entries: Array<{
    timestamp: string;
    session_id: string;
    action: string;
    tool?: string;
    role: string;
    detail: string;
  }>;
}

interface ZeusRbacScopes {
  role: string;
  can_execute_mutations: boolean;
  tools: Array<{ name: string; risk: string; allowed_roles: string[] }>;
}

interface PolicyExplainerReport {
  summary: string;
  plain_english: string[];
}

interface RunbookAuthorReport {
  title: string;
  markdown: string;
}

export default function ZeusPlatformPanel() {
  const [tab, setTab] = useState<
    'memory' | 'route' | 'llm' | 'voice' | 'runbook' | 'policy' | 'audit' | 'rbac'
  >('memory');
  const [loading, setLoading] = useState(true);
  const [memory, setMemory] = useState<ZeusMemoryReport | null>(null);
  const [llm, setLlm] = useState<LlmProviderStatus | null>(null);
  const [voice, setVoice] = useState<VoiceLab | null>(null);
  const [audit, setAudit] = useState<ZeusAuditReport | null>(null);
  const [rbac, setRbac] = useState<ZeusRbacScopes | null>(null);
  const [routeMessage, setRouteMessage] = useState('Find cost savings across the fleet');
  const [route, setRoute] = useState<MultiAgentRoute | null>(null);
  const [runbookPrompt, setRunbookPrompt] = useState('Restart unhealthy pods safely');
  const [runbook, setRunbook] = useState<RunbookAuthorReport | null>(null);
  const [policy, setPolicy] = useState<PolicyExplainerReport | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [mem, llmStatus, voiceLab, auditTrail, scopes] = await Promise.all([
      apiFetch<ZeusMemoryReport>('/intelligence/zeus/memory'),
      apiFetch<LlmProviderStatus>('/intelligence/zeus/llm-status'),
      apiFetch<VoiceLab>('/intelligence/zeus/voice-lab'),
      apiFetch<ZeusAuditReport>('/intelligence/zeus/audit'),
      apiFetch<ZeusRbacScopes>('/intelligence/zeus/rbac-scopes'),
    ]);
    setMemory(mem);
    setLlm(llmStatus);
    setVoice(voiceLab);
    setAudit(auditTrail);
    setRbac(scopes);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function routeAgent() {
    const res = await apiPost<MultiAgentRoute>('/intelligence/zeus/route', {
      message: routeMessage.trim(),
    });
    if (res.success) setRoute(res.data ?? null);
  }

  async function authorRunbook() {
    const res = await apiPost<RunbookAuthorReport>('/intelligence/zeus/runbook', {
      prompt: runbookPrompt.trim(),
    });
    if (res.success) setRunbook(res.data ?? null);
  }

  async function explainPolicy() {
    const res = await apiPost<PolicyExplainerReport>('/intelligence/zeus/policy-explain', {});
    if (res.success) setPolicy(res.data ?? null);
  }

  const tabs = [
    { id: 'memory' as const, label: 'Memory' },
    { id: 'route' as const, label: 'Agents' },
    { id: 'llm' as const, label: 'LLM' },
    { id: 'voice' as const, label: 'Voice Lab' },
    { id: 'runbook' as const, label: 'Runbooks' },
    { id: 'policy' as const, label: 'Policy' },
    { id: 'audit' as const, label: 'Audit' },
    { id: 'rbac' as const, label: 'RBAC' },
  ];

  return (
    <GlassSection
      accent="purple"
      testId="zeus-platform-panel"
      title="Zeus Platform"
      subtitle="Memory, multi-agent routing, LLM status, runbooks, policy explain, audit, RBAC"
      icon={<Bot className="h-5 w-5 text-violet-400" />}
      actions={
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-slate-300 hover:border-aether-ai/40"
        >
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          Refresh
        </button>
      }
    >
      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-violet-500/40 bg-violet-500/10 text-violet-200' : 'glass-divider text-slate-400'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'memory' ? (
        <div data-testid="zeus-memory-panel">
          <p className="mb-3 text-xs text-slate-500">
            Persisted: {memory?.persisted ? 'yes' : 'no'} · {memory?.entries.length ?? 0} session(s)
          </p>
          {!memory?.entries.length ? (
            <p className="text-sm text-slate-500">No zeus memory yet — chat to populate session summaries.</p>
          ) : (
            <ul className="space-y-2">
              {memory.entries.map((e) => (
                <li key={e.session_id} className="rounded-lg border glass-divider px-3 py-2 text-sm">
                  <p className="font-mono text-xs text-violet-300">{e.session_id.slice(0, 8)}…</p>
                  <p className="mt-1 text-slate-300">{e.summary}</p>
                  <p className="text-xs text-slate-500">{e.updated_at}</p>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}

      {tab === 'route' ? (
        <div data-testid="zeus-route-panel">
          <div className="mb-3 flex flex-wrap gap-2">
            <input
              value={routeMessage}
              onChange={(e) => setRouteMessage(e.target.value)}
              className="glass-input min-w-[240px] flex-1"
              data-testid="zeus-route-input"
            />
            <button
              type="button"
              onClick={() => void routeAgent()}
              className="rounded-lg bg-violet-600 px-3 py-2 text-xs text-white hover:bg-violet-500"
              data-testid="zeus-route-button"
            >
              Route agent
            </button>
          </div>
          {route ? (
            <div className="rounded-lg border glass-divider px-3 py-3 text-sm">
              <p className="font-medium text-white">
                {route.label} <span className="text-slate-500">({route.agent})</span>
              </p>
              <p className="text-xs text-slate-400">Confidence: {(route.confidence * 100).toFixed(0)}%</p>
              <ul className="mt-2 list-disc pl-5 text-xs text-slate-400">
                {route.suggested_prompts.map((p) => (
                  <li key={p}>{p}</li>
                ))}
              </ul>
            </div>
          ) : (
            <p className="text-sm text-slate-500">Enter a message to route to SRE, FinOps, or specialist agents.</p>
          )}
        </div>
      ) : null}

      {tab === 'llm' ? (
        <div data-testid="zeus-llm-panel" className="space-y-2 text-sm">
          <p>
            Active provider: <span className="font-medium text-violet-200">{llm?.active_provider ?? '—'}</span>
          </p>
          <p className="text-slate-400">Model: {llm?.model ?? '—'}</p>
          <p className="text-xs text-slate-500">
            OpenAI: {llm?.openai_configured ? 'yes' : 'no'} · Anthropic: {llm?.anthropic_configured ? 'yes' : 'no'} ·
            Ollama: {llm?.ollama_configured ? 'yes' : 'no'}
          </p>
          {llm?.fallback_rule_based ? (
            <p className="text-xs text-amber-300/90">Rule-based fallback active when no LLM keys are configured.</p>
          ) : null}
        </div>
      ) : null}

      {tab === 'voice' ? (
        <div data-testid="zeus-voice-lab-panel" className="space-y-2 text-sm">
          <div className="flex items-center gap-2 text-violet-200">
            <Mic className="h-4 w-4" />
            Voice zeus lab ({voice?.status ?? '—'})
          </div>
          <p className="text-slate-400">{voice?.hint}</p>
          <p className="text-xs text-slate-500">Sample: {voice?.sample_transcript}</p>
        </div>
      ) : null}

      {tab === 'runbook' ? (
        <div data-testid="zeus-runbook-panel">
          <div className="mb-3 flex flex-wrap gap-2">
            <input
              value={runbookPrompt}
              onChange={(e) => setRunbookPrompt(e.target.value)}
              className="glass-input min-w-[240px] flex-1"
              data-testid="zeus-runbook-input"
            />
            <button
              type="button"
              onClick={() => void authorRunbook()}
              className="inline-flex items-center gap-1 rounded-lg bg-violet-600 px-3 py-2 text-xs text-white hover:bg-violet-500"
              data-testid="zeus-runbook-button"
            >
              <Sparkles className="h-3.5 w-3.5" />
              Author
            </button>
          </div>
          {runbook ? (
            <pre className="glass-code-block-body max-h-64 text-xs text-slate-300 whitespace-pre-wrap">
              {runbook.markdown}
            </pre>
          ) : (
            <p className="text-sm text-slate-500">Describe an incident or procedure to generate a markdown runbook.</p>
          )}
        </div>
      ) : null}

      {tab === 'policy' ? (
        <div data-testid="zeus-policy-panel">
          <button
            type="button"
            onClick={() => void explainPolicy()}
            className="mb-3 inline-flex items-center gap-1 rounded-lg border glass-divider px-3 py-2 text-xs text-slate-300 hover:border-violet-500/40"
            data-testid="zeus-policy-button"
          >
            <Shield className="h-3.5 w-3.5" />
            Explain violations
          </button>
          {policy ? (
            <div className="space-y-2 text-sm">
              <p className="text-slate-300">{policy.summary}</p>
              {!policy.plain_english.length ? (
                <p className="text-slate-500">No violations in current fleet snapshot.</p>
              ) : (
                <ul className="list-disc pl-5 text-xs text-slate-400">
                  {policy.plain_english.map((line) => (
                    <li key={line}>{line}</li>
                  ))}
                </ul>
              )}
            </div>
          ) : (
            <p className="text-sm text-slate-500">Plain-English summary of OPA and intent policy violations.</p>
          )}
        </div>
      ) : null}

      {tab === 'audit' ? (
        <div data-testid="zeus-audit-panel">
          {!audit?.entries.length ? (
            <p className="text-sm text-slate-500">No zeus audit entries yet.</p>
          ) : (
            <ul className="max-h-64 space-y-2 overflow-y-auto">
              {audit.entries.map((e, i) => (
                <li key={`${e.timestamp}-${i}`} className="rounded-lg border glass-divider px-3 py-2 text-xs">
                  <span className="text-violet-300">{e.action}</span>
                  {e.tool ? <span className="text-slate-500"> · {e.tool}</span> : null}
                  <p className="text-slate-400">{e.detail}</p>
                  <p className="text-slate-600">{e.timestamp}</p>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}

      {tab === 'rbac' ? (
        <div data-testid="zeus-rbac-panel">
          <p className="mb-2 text-sm text-slate-300">
            Role: {rbac?.role ?? '—'} · Mutations: {rbac?.can_execute_mutations ? 'allowed' : 'denied'}
          </p>
          <ul className="max-h-64 space-y-1 overflow-y-auto text-xs">
            {(rbac?.tools ?? []).slice(0, 12).map((t) => (
              <li key={t.name} className="flex justify-between gap-2 glass-table-row py-1 text-slate-400">
                <span className="font-mono text-slate-300">{t.name}</span>
                <span>{t.risk}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </GlassSection>
  );
}
