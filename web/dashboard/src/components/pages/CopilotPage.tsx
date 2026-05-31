// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useRef, useState } from 'react';
import { Link } from 'react-router';
import { Bot, Send, Sparkles } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { apiPost } from '../../utils/api';
import CopilotPlatformPanel from '../CopilotPlatformPanel';

interface ToolResult {
  tool: string;
  summary: string;
}

interface PendingAction {
  id: string;
  tool: string;
  description: string;
}

interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
}

const SUGGESTIONS = [
  'Why is my workload unhealthy?',
  'Diagnose payment-service with live cluster evidence',
  'Show AI insights: predictions, threats, and cost',
  'List all workloads and their runtimes',
  'What policy violations exist in production?',
  'Show failure predictions for the fleet',
  'What cost optimizations are available?',
  'Summarize GitOps sync status',
  'Generate a security hardening plan',
];

export default function CopilotPage() {
  const [workloadParam] = useQueryParam('workload', '');
  const [qParam, setQParam] = useQueryParam('q', '');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingAction[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const prefill = qParam.trim();
    if (!prefill) return;
    setInput((current) => (current.trim() ? current : prefill));
  }, [qParam]);

  useEffect(() => {
    const focus = workloadParam.trim();
    if (!focus) return;
    const prompt = `Summarize health and trust posture for ${focus}`;
    setInput((current) => (current.trim() ? current : prompt));
  }, [workloadParam]);

  const send = useCallback(
    async (text: string, confirmActionId?: string) => {
      const trimmed = text.trim();
      if (!trimmed || loading) return;

      setMessages((m) => [...m, { role: 'user', content: trimmed }]);
      setInput('');
      setLoading(true);

      try {
        const res = await apiPost<{
          session_id: string;
          reply: string;
          tool_results: ToolResult[];
          pending_actions: PendingAction[];
        }>('/copilot/chat', {
          message: trimmed,
          session_id: sessionId,
          confirm_action_id: confirmActionId,
        });

        if (res.success && res.data) {
          setSessionId(res.data.session_id);
          const toolNotes =
            res.data.tool_results?.length > 0
              ? '\n\n' + res.data.tool_results.map((t) => `[${t.tool}] ${t.summary}`).join('\n')
              : '';
          setMessages((m) => [
            ...m,
            { role: 'assistant', content: (res.data?.reply ?? '') + toolNotes },
          ]);
          setPending(res.data.pending_actions ?? []);
        } else {
          setMessages((m) => [
            ...m,
            { role: 'assistant', content: res.error ?? 'Copilot request failed' },
          ]);
        }
      } catch (e) {
        setMessages((m) => [
          ...m,
          { role: 'assistant', content: e instanceof Error ? e.message : 'Network error' },
        ]);
      } finally {
        setLoading(false);
        setTimeout(() => bottomRef.current?.scrollIntoView({ behavior: 'smooth' }), 50);
      }
    },
    [loading, sessionId],
  );

  const confirmAction = (actionId: string) => {
    void send('confirm', actionId);
    setPending((p) => p.filter((a) => a.id !== actionId));
  };

  const confirmBatch = async () => {
    if (!sessionId || pending.length === 0 || loading) return;
    setLoading(true);
    try {
      const res = await apiPost<{
        session_id: string;
        confirmed: string[];
        skipped: string[];
        errors: string[];
      }>('/copilot/confirm-batch', {
        session_id: sessionId,
        action_ids: pending.map((a) => a.id),
      });
      if (res.success && res.data) {
        const { confirmed, skipped, errors } = res.data;
        const summary = [
          confirmed.length ? `Confirmed ${confirmed.length} action(s).` : '',
          skipped.length ? `Skipped ${skipped.length}.` : '',
          errors.length ? errors.join('\n') : '',
        ]
          .filter(Boolean)
          .join('\n');
        if (summary) {
          setMessages((m) => [...m, { role: 'assistant', content: summary }]);
        }
        setPending((p) => p.filter((a) => !confirmed.includes(a.id)));
      }
    } finally {
      setLoading(false);
    }
  };

  const workloadFocus = workloadParam.trim();

  return (
    <div className="flex h-[calc(100vh-12rem)] min-h-[480px] flex-col gap-4">
      <WorkloadContextBanner
        testId="copilot-workload-context"
        workload={workloadFocus}
        description="Copilot context for workload"
      >
        <WorkloadScopedCrossLinks
          workload={workloadFocus}
          prefix="copilot"
          showDrift
          showAudit
          showGitops
          showMetrics
        />
        {workloadFocus ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('intelligence'), { workload: workloadFocus, tab: 'predictions' })}
              className="text-aether hover:underline"
              data-testid="copilot-context-intelligence-link"
            >
              Intelligence →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: workloadFocus })}
              className="text-aether hover:underline"
              data-testid="copilot-editor-link"
            >
              Editor →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: workloadFocus })}
              className="text-aether hover:underline"
              data-testid="copilot-context-openapi-link"
            >
              OpenAPI →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: workloadFocus })}
              className="text-aether hover:underline"
              data-testid="copilot-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: workloadFocus })}
              className="text-aether hover:underline"
              data-testid="copilot-context-secrets-link"
            >
              Secrets →
            </Link>
          </>
        ) : null}
      </WorkloadContextBanner>
      <div className="copilot-rail-glass relative flex flex-1 flex-col overflow-hidden rounded-xl border">
        <div className="relative z-[1] flex items-center gap-3 border-b border-slate-800/50 px-4 py-4">
          <div className="flex h-9 w-9 items-center justify-center rounded-xl border border-aether-ai/30 bg-gradient-to-br from-aether/20 to-aether-ai/20">
            <Bot className="h-4 w-4 text-[#c084fc]" aria-hidden />
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="text-sm font-semibold text-white">AI Ops Copilot</h2>
            <p className="text-[10px] font-medium uppercase tracking-[0.18em] text-aether-ai">
              Infrastructure agent
            </p>
          </div>
          {messages.length > 0 && (
            <button
              type="button"
              data-testid="copilot-clear-chat"
              onClick={() => {
                setMessages([]);
                setSessionId(null);
                setPending([]);
              }}
              className="rounded-lg border border-slate-700/80 px-2 py-1 text-xs text-slate-400 transition hover:border-slate-600 hover:text-slate-200"
            >
              Clear chat
            </button>
          )}
          {messages.length === 0 && (
            <span className="text-xs text-slate-500">Natural language control plane</span>
          )}
          <Link
            to={viewToPath('intelligence')}
            className="text-xs text-aether hover:underline"
            data-testid="copilot-intelligence-link"
          >
            Intelligence reports →
          </Link>
          <Link
            to={
              workloadFocus
                ? pathWithQuery(viewToPath('health'), { workload: workloadFocus })
                : viewToPath('health')
            }
            className="text-xs text-aether hover:underline ml-3"
            data-testid="copilot-health-link"
          >
            Health monitor →
          </Link>
          {workloadFocus ? (
            <>
              <Link
                to={pathWithQuery(viewToPath('workloads'), { workload: workloadFocus, tab: 'trust' })}
                className="text-xs text-aether hover:underline ml-3"
                data-testid="copilot-trust-link"
              >
                Trust tab →
              </Link>
              <Link
                to={pathWithQuery(viewToPath('alerts'), { workload: workloadFocus })}
                className="text-xs text-aether hover:underline ml-3"
                data-testid="copilot-alerts-link"
              >
                Alert rules →
              </Link>
              <Link
                to={pathWithQuery(viewToPath('events'), { workload: workloadFocus })}
                className="text-xs text-aether hover:underline ml-3"
                data-testid="copilot-events-link"
              >
                Events →
              </Link>
            </>
          ) : null}
        </div>

        <div className="relative z-[1] flex-1 space-y-3 overflow-y-auto p-4">
          {messages.length === 0 && (
            <div className="rounded-2xl border border-aether-ai/15 bg-[#161B24] px-4 py-6 text-center">
              <Sparkles className="mx-auto mb-3 h-8 w-8 text-[#c084fc]/80" aria-hidden />
              <p className="text-sm text-slate-400">
                Ask about health, drift, costs, migrations, or cluster state.
              </p>
              <div className="mt-4 flex flex-wrap justify-center gap-2" data-testid="copilot-suggestions">
                {SUGGESTIONS.map((s) => (
                  <button
                    key={s}
                    type="button"
                    data-testid="copilot-suggestion"
                    onClick={() => {
                      setQParam(s);
                      setInput(s);
                      void send(s);
                    }}
                    className="copilot-prompt-chip"
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>
          )}

          {messages.map((msg, i) => (
            <div
              key={`${msg.role}-${i}`}
              className={`max-w-[85%] rounded-2xl px-4 py-2.5 text-sm whitespace-pre-wrap ${
                msg.role === 'user'
                  ? 'ml-auto border border-aether-ai/20 bg-gradient-to-br from-aether/20 to-aether-ai/15 text-violet-50'
                  : 'border border-slate-800/60 bg-[#161B24]/80 text-slate-200'
              }`}
            >
              {msg.content}
            </div>
          ))}

          {pending.length > 0 && (
            <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3" data-testid="copilot-pending-actions">
              <div className="mb-2 flex items-center justify-between gap-2">
                <p className="text-xs font-medium text-amber-200">Actions awaiting confirmation</p>
                {pending.length > 1 ? (
                  <button
                    type="button"
                    data-testid="copilot-confirm-batch"
                    onClick={() => void confirmBatch()}
                    className="rounded-lg bg-amber-600 px-2 py-1 text-xs text-white hover:bg-amber-500"
                  >
                    Approve all ({pending.length})
                  </button>
                ) : null}
              </div>
              {pending.map((a) => (
                <div key={a.id} className="flex items-center justify-between gap-2 py-1 text-xs text-slate-300">
                  <span>{a.description}</span>
                  <button
                    type="button"
                    onClick={() => confirmAction(a.id)}
                    className="rounded-lg bg-amber-600 px-2 py-1 text-white hover:bg-amber-500"
                  >
                    Confirm
                  </button>
                </div>
              ))}
            </div>
          )}

          {loading && (
            <div className="flex items-center gap-2 text-xs text-slate-500">
              <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-aether-ai" />
              Copilot is thinking…
            </div>
          )}
          <div ref={bottomRef} />
        </div>

        <form
          className="relative z-[1] flex gap-2 border-t border-slate-800/50 p-3"
          onSubmit={(e) => {
            e.preventDefault();
            void send(input);
          }}
        >
          <input
            value={input}
            onChange={(e) => {
              setInput(e.target.value);
              setQParam(e.target.value);
            }}
            placeholder="Ask Aether anything…"
            data-testid="copilot-input"
            className="min-w-0 flex-1 rounded-xl border border-slate-700/70 bg-[#11151C]/80 px-4 py-2.5 text-sm text-slate-100 outline-none transition focus:border-aether-ai/45 focus:ring-1 focus:ring-aether-ai/20"
            disabled={loading}
          />
          <button
            type="submit"
            data-testid="copilot-send-button"
            disabled={loading || !input.trim()}
            className="flex items-center gap-2 rounded-xl bg-gradient-to-br from-aether to-aether-ai px-4 py-2.5 text-sm font-medium text-white shadow-lg shadow-aether-ai/20 transition hover:opacity-90 disabled:opacity-50"
          >
            <Send className="h-4 w-4" aria-hidden />
            Send
          </button>
        </form>
      </div>

      <CopilotPlatformPanel />
    </div>
  );
}
