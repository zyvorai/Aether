// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useRef, useState } from 'react';
import { Bot, Send, Sparkles } from 'lucide-react';
import { apiPost } from '../../utils/api';

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
  'List all workloads and their runtimes',
  'Show failure predictions for the fleet',
  'What cost optimizations are available?',
  'Summarize platform context',
];

export default function CopilotPage() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingAction[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);

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

  return (
    <div className="flex h-[calc(100vh-12rem)] min-h-[480px] flex-col gap-4">
      <div className="dash-card flex flex-1 flex-col overflow-hidden p-0">
        <div className="flex items-center gap-2 border-b border-slate-800/60 px-4 py-3">
          <Bot className="h-5 w-5 text-violet-400" aria-hidden />
          <h2 className="text-sm font-semibold text-slate-100">AI Ops Copilot</h2>
          <span className="ml-auto text-xs text-slate-500">Natural language control plane</span>
        </div>

        <div className="flex-1 space-y-3 overflow-y-auto p-4">
          {messages.length === 0 && (
            <div className="rounded-xl border border-dashed border-slate-700/60 bg-slate-900/40 p-6 text-center">
              <Sparkles className="mx-auto mb-3 h-8 w-8 text-violet-400/80" aria-hidden />
              <p className="text-sm text-slate-400">
                Ask about health, drift, costs, migrations, or cluster state.
              </p>
              <div className="mt-4 flex flex-wrap justify-center gap-2">
                {SUGGESTIONS.map((s) => (
                  <button
                    key={s}
                    type="button"
                    onClick={() => void send(s)}
                    className="rounded-full border border-slate-700/80 bg-slate-800/60 px-3 py-1 text-xs text-slate-300 hover:border-violet-500/40 hover:text-violet-200"
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
                  ? 'ml-auto bg-violet-600/30 text-violet-50'
                  : 'bg-slate-800/80 text-slate-200'
              }`}
            >
              {msg.content}
            </div>
          ))}

          {pending.length > 0 && (
            <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-3" data-testid="copilot-pending-actions">
              <p className="mb-2 text-xs font-medium text-amber-200">Actions awaiting confirmation</p>
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
            <div className="text-xs text-slate-500 animate-pulse">Copilot is thinking…</div>
          )}
          <div ref={bottomRef} />
        </div>

        <form
          className="flex gap-2 border-t border-slate-800/60 p-3"
          onSubmit={(e) => {
            e.preventDefault();
            void send(input);
          }}
        >
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Ask Aether anything…"
            className="flex-1 rounded-xl border border-slate-700/80 bg-slate-900/60 px-4 py-2.5 text-sm text-slate-100 outline-none focus:border-violet-500/50"
            disabled={loading}
          />
          <button
            type="submit"
            disabled={loading || !input.trim()}
            className="flex items-center gap-2 rounded-xl bg-violet-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-violet-500 disabled:opacity-50"
          >
            <Send className="h-4 w-4" aria-hidden />
            Send
          </button>
        </form>
      </div>
    </div>
  );
}
