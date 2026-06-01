// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useMemo, useRef, useState } from 'react';
import { useLocation } from 'react-router';
import {
  Bot,
  ChevronLeft,
  ChevronRight,
  DollarSign,
  GitBranch,
  HeartPulse,
  Rocket,
  Send,
  Shield,
  Sparkles,
} from 'lucide-react';
import { apiFetch } from '../utils/api';
import { isMacOSShell } from '../utils/macosBridge';
import { useQueryParam } from '../utils/urlState';
import { useZeusChat } from '../hooks/useZeusChat';
import type { AgentStatusEntry } from '../types/api';

interface ZeusRailProps {
  collapsed?: boolean;
  onCollapsedChange?: (collapsed: boolean) => void;
}

const AGENT_ICONS: Record<string, typeof Bot> = {
  sre: HeartPulse,
  cost: DollarSign,
  security: Shield,
  capacity: Bot,
  migration: Rocket,
  gitops: GitBranch,
};

const AGENT_PROMPTS: Record<string, string[]> = {
  sre: [
    'Run fleet health diagnosis',
    'Which workloads need healing?',
    'Why did latency increase yesterday?',
    'Show all SLA violations',
  ],
  cost: [
    'Find workloads wasting resources',
    'Predict cost next month',
    'Move frontend to cheapest runtime',
    'Show top cost drivers',
  ],
  security: [
    'Scan fleet for security threats',
    'Show policy violations',
    'Which workloads have exposed secrets?',
    'Review trust attestation status',
  ],
  capacity: [
    'Predict capacity risks this week',
    'Which resources will saturate first?',
    'Recommend scaling actions',
    'Show utilization hotspots',
  ],
  migration: [
    'Show migration opportunities',
    'Recommend runtime placement',
    'Which workloads should move to KubeVirt?',
    'Compare placement options',
  ],
  gitops: [
    'Check GitOps drift across fleet',
    'Show sync failures',
    'Which repos are out of sync?',
    'Summarize reconciliation status',
  ],
};

const DEFAULT_PROMPTS = [
  'Summarize fleet health',
  'Find workloads wasting resources',
  'Show all SLA violations',
  'Predict cost next month',
  'Why did latency increase yesterday?',
];

function agentStatusDot(status: string): string {
  if (status === 'alert') return 'bg-red-400 shadow-[0_0_8px_rgba(248,113,113,0.6)]';
  if (status === 'active') return 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.5)]';
  return 'glass-status-dot-muted';
}

export default function ZeusRail({ collapsed: controlledCollapsed, onCollapsedChange }: ZeusRailProps) {
  const location = useLocation();
  const [workloadParam] = useQueryParam('workload', '');
  const [internalCollapsed, setInternalCollapsed] = useState(false);
  const collapsed = controlledCollapsed ?? internalCollapsed;
  const setCollapsed = onCollapsedChange ?? setInternalCollapsed;
  const bottomRef = useRef<HTMLDivElement>(null);
  const [agents, setAgents] = useState<AgentStatusEntry[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const {
    messages,
    input,
    setInput,
    loading,
    pending,
    send,
    confirmAction,
    clearChat,
  } = useZeusChat({
    route: location.pathname,
    workload: workloadParam,
    agentFocus: selectedAgent,
  });

  useEffect(() => {
    let cancelled = false;
    apiFetch<{ agents: AgentStatusEntry[] }>('/intelligence/agents/status').then((report) => {
      if (!cancelled && report?.agents) {
        setAgents(report.agents);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const visiblePrompts = useMemo(() => {
    if (selectedAgent && AGENT_PROMPTS[selectedAgent]) {
      return AGENT_PROMPTS[selectedAgent];
    }
    return DEFAULT_PROMPTS;
  }, [selectedAgent]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, loading]);

  const railVisibility = isMacOSShell() ? 'flex' : 'hidden xl:flex';

  if (collapsed) {
    return (
      <aside
        className={`zeus-rail-glass relative ${railVisibility} w-12 shrink-0 flex-col items-center border-l py-4`}
        data-testid="zeus-rail-collapsed"
      >
        <button
          type="button"
          onClick={() => setCollapsed(false)}
          className="rounded-xl border border-aether-ai/30 bg-aether-ai/10 p-2 text-[#c084fc] transition hover:bg-aether-ai/20"
          title="Open Ask Aether"
          aria-label="Open Ask Aether zeus"
        >
          <Bot className="h-5 w-5" />
        </button>
      </aside>
    );
  }

  return (
    <aside
      className={`zeus-rail-glass relative ${railVisibility} w-[min(380px,30vw)] shrink-0 flex-col border-l`}
      data-testid="zeus-rail"
    >
      <div className="relative z-[1] flex items-center gap-3 glass-table-row px-4 py-4">
        <div className="flex h-9 w-9 items-center justify-center rounded-xl border border-aether-ai/30 bg-gradient-to-br from-aether/20 to-aether-ai/20">
          <Sparkles className="h-4 w-4 text-[#c084fc]" aria-hidden />
        </div>
        <div className="min-w-0 flex-1">
          <div className="text-sm font-semibold text-white">Zeus</div>
          <div className="text-[10px] font-medium uppercase tracking-[0.18em] text-aether-ai">
            Infrastructure OS assistant
          </div>
        </div>
        {messages.length > 0 ? (
          <button
            type="button"
            onClick={clearChat}
            className="rounded-lg border glass-divider px-2 py-1 text-[10px] text-slate-400 transition hover:border-aether/30 hover:text-slate-200"
          >
            Clear
          </button>
        ) : null}
        <button
          type="button"
          onClick={() => setCollapsed(true)}
          className="rounded-lg p-1.5 text-slate-500 transition glass-inset-hover hover:text-slate-200"
          title="Collapse zeus"
          aria-label="Collapse zeus"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      </div>

      {agents.length > 0 ? (
        <div className="relative z-[1] glass-table-row px-3 py-3">
          <p className="mb-2 px-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-500">
            Agent focus
          </p>
          <div className="grid grid-cols-2 gap-1.5">
            {agents.map((agent) => {
              const Icon = AGENT_ICONS[agent.id] ?? Bot;
              const active = selectedAgent === agent.id;
              return (
                <button
                  key={agent.id}
                  type="button"
                  onClick={() => setSelectedAgent(active ? null : agent.id)}
                  className={`zeus-agent-chip ${active ? 'zeus-agent-chip-active' : ''}`}
                  data-testid={`zeus-agent-${agent.id}`}
                >
                  <div className="flex items-center gap-2">
                    <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${agentStatusDot(agent.status)}`} />
                    <Icon className="h-3 w-3 shrink-0 text-slate-400" />
                    <span className="truncate text-[11px] font-medium text-slate-200">{agent.label}</span>
                  </div>
                  {agent.pending_count > 0 ? (
                    <div className="mt-1 pl-3.5 text-[10px] text-amber-400/90">{agent.pending_count} pending</div>
                  ) : (
                    <div className="mt-1 truncate pl-3.5 text-[10px] text-slate-500">{agent.detail}</div>
                  )}
                </button>
              );
            })}
          </div>
        </div>
      ) : null}

      <div className="relative z-[1] flex-1 space-y-2 overflow-y-auto p-3">
        {messages.length === 0 ? (
          <div className="space-y-4">
            <div className="rounded-2xl border border-aether-ai/15 glass-inset-surface px-3 py-3 backdrop-blur-sm">
              <p className="text-xs leading-relaxed text-slate-400">
                Not a chatbot — an infrastructure co-pilot. Ask about health, cost, migrations, security, or
                capacity.
              </p>
            </div>
            <div>
              <p className="mb-2 px-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-500">
                Suggested questions
              </p>
              <div className="flex flex-col gap-1.5">
                {visiblePrompts.map((prompt) => (
                  <button
                    key={prompt}
                    type="button"
                    onClick={() => void send(prompt)}
                    className="zeus-prompt-chip"
                    data-testid={`zeus-prompt-${prompt.slice(0, 20).replace(/\s+/g, '-').toLowerCase()}`}
                  >
                    {prompt}
                  </button>
                ))}
              </div>
            </div>
          </div>
        ) : null}

        {messages.map((msg, i) => (
          <div
            key={`${msg.role}-${i}`}
            className={`max-w-full rounded-2xl px-3 py-2.5 text-xs leading-relaxed whitespace-pre-wrap ${
              msg.role === 'user'
                ? 'ml-6 border border-aether-ai/20 bg-gradient-to-br from-aether/20 to-aether-ai/15 text-violet-50'
                : 'mr-2 border glass-divider glass-inset-surface text-slate-200'
            }`}
          >
            {msg.content}
          </div>
        ))}

        {pending.length > 0 ? (
          <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-2.5 backdrop-blur-sm">
            <p className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-amber-200">
              Awaiting confirmation
            </p>
            {pending.map((a) => (
              <div key={a.id} className="flex items-center justify-between gap-2 py-1 text-[10px] text-slate-300">
                <span className="min-w-0 truncate">{a.description}</span>
                <button
                  type="button"
                  onClick={() => confirmAction(a.id)}
                  className="shrink-0 rounded-lg bg-amber-600 px-2 py-0.5 font-medium text-white transition hover:bg-amber-500"
                >
                  Confirm
                </button>
              </div>
            ))}
          </div>
        ) : null}

        {loading ? (
          <div className="flex items-center gap-2 px-1 text-[10px] text-slate-500">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-aether-ai" />
            Thinking…
          </div>
        ) : null}
        <div ref={bottomRef} />
      </div>

      <form
        className="relative z-[1] flex gap-2 glass-divider-t/50 p-3"
        onSubmit={(e) => {
          e.preventDefault();
          void send(input);
        }}
      >
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={selectedAgent ? `Ask ${agents.find((a) => a.id === selectedAgent)?.label ?? 'Aether'}…` : 'Ask Aether…'}
          data-testid="zeus-rail-input"
          className="glass-input min-w-0 flex-1 text-xs"
          disabled={loading}
        />
        <button
          type="submit"
          disabled={loading || !input.trim()}
          className="rounded-xl bg-gradient-to-br from-aether to-aether-ai p-2.5 text-white shadow-lg shadow-aether-ai/20 transition hover:opacity-90 disabled:opacity-40"
          aria-label="Send"
        >
          <Send className="h-4 w-4" />
        </button>
      </form>
    </aside>
  );
}

/** Mobile/tablet toggle button shown below xl breakpoint */
export function ZeusRailToggle({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="fixed bottom-6 right-6 z-30 flex items-center gap-2 rounded-full border border-aether-ai/40 bg-gradient-to-r from-aether to-aether-ai px-4 py-2.5 text-sm font-medium text-white shadow-lg shadow-aether-ai/25 backdrop-blur-xl xl:hidden"
      data-testid="zeus-rail-mobile-toggle"
    >
      <Bot className="h-4 w-4" />
      Ask Aether
      <ChevronLeft className="h-4 w-4 opacity-70" />
    </button>
  );
}
