// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useState } from 'react';
import { apiPost } from '../utils/api';

export interface ZeusChatMessage {
  role: 'user' | 'assistant';
  content: string;
}

export interface ZeusPendingAction {
  id: string;
  tool: string;
  description: string;
}

export interface ZeusToolResult {
  tool: string;
  summary: string;
}

export interface ZeusChatContext {
  route?: string;
  workload?: string;
  agentFocus?: string | null;
}

const QUICK_PROMPTS = [
  'Move frontend to cheapest runtime',
  'Find workloads wasting resources',
  'Show all SLA violations',
  'Why did latency increase yesterday?',
  'Predict cost next month',
];

function buildContextualMessage(text: string, context?: ZeusChatContext): string {
  const parts: string[] = [];
  if (context?.route) parts.push(`[Route: ${context.route}]`);
  if (context?.workload?.trim()) parts.push(`[Workload: ${context.workload.trim()}]`);
  if (context?.agentFocus) parts.push(`[Agent: ${context.agentFocus}]`);
  if (parts.length === 0) return text;
  return `${parts.join(' ')}\n\n${text}`;
}

export function useZeusChat(context?: ZeusChatContext) {
  const [messages, setMessages] = useState<ZeusChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [pending, setPending] = useState<ZeusPendingAction[]>([]);

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
          tool_results: ZeusToolResult[];
          pending_actions: ZeusPendingAction[];
        }>('/zeus/chat', {
          message: buildContextualMessage(trimmed, context),
          session_id: sessionId,
          confirm_action_id: confirmActionId,
          agent_focus: context?.agentFocus ?? undefined,
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
            { role: 'assistant', content: res.error ?? 'Zeus request failed' },
          ]);
        }
      } catch (e) {
        setMessages((m) => [
          ...m,
          { role: 'assistant', content: e instanceof Error ? e.message : 'Network error' },
        ]);
      } finally {
        setLoading(false);
      }
    },
    [context, loading, sessionId],
  );

  const confirmAction = useCallback(
    (actionId: string) => {
      void send('confirm', actionId);
      setPending((p) => p.filter((a) => a.id !== actionId));
    },
    [send],
  );

  const confirmBatch = useCallback(async () => {
    if (!sessionId || pending.length === 0 || loading) return;
    setLoading(true);
    try {
      const res = await apiPost<{
        session_id: string;
        confirmed: string[];
        skipped: string[];
        errors: string[];
      }>('/zeus/confirm-batch', {
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
  }, [loading, pending, sessionId]);

  const clearChat = useCallback(() => {
    setMessages([]);
    setSessionId(null);
    setPending([]);
  }, []);

  return {
    messages,
    input,
    setInput,
    loading,
    sessionId,
    pending,
    send,
    confirmAction,
    confirmBatch,
    clearChat,
    quickPrompts: QUICK_PROMPTS,
  };
}
