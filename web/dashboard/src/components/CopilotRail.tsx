// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useRef, useState } from 'react';
import { Bot, ChevronLeft, ChevronRight, Send, Sparkles } from 'lucide-react';
import { useCopilotChat } from '../hooks/useCopilotChat';

interface CopilotRailProps {
  collapsed?: boolean;
  onCollapsedChange?: (collapsed: boolean) => void;
}

export default function CopilotRail({ collapsed: controlledCollapsed, onCollapsedChange }: CopilotRailProps) {
  const [internalCollapsed, setInternalCollapsed] = useState(false);
  const collapsed = controlledCollapsed ?? internalCollapsed;
  const setCollapsed = onCollapsedChange ?? setInternalCollapsed;
  const bottomRef = useRef<HTMLDivElement>(null);
  const {
    messages,
    input,
    setInput,
    loading,
    pending,
    send,
    confirmAction,
    clearChat,
    quickPrompts,
  } = useCopilotChat();

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, loading]);

  if (collapsed) {
    return (
      <aside
        className="hidden xl:flex w-12 shrink-0 flex-col items-center border-l border-slate-800/60 bg-slate-950/40 backdrop-blur-xl py-4"
        data-testid="copilot-rail-collapsed"
      >
        <button
          type="button"
          onClick={() => setCollapsed(false)}
          className="rounded-xl border border-violet-500/30 bg-violet-500/10 p-2 text-violet-300 hover:bg-violet-500/20"
          title="Open Ask Aether"
          aria-label="Open Ask Aether copilot"
        >
          <Bot className="h-5 w-5" />
        </button>
      </aside>
    );
  }

  return (
    <aside
      className="hidden xl:flex w-[min(360px,28vw)] shrink-0 flex-col border-l border-slate-800/60 bg-slate-950/55 backdrop-blur-xl"
      data-testid="copilot-rail"
    >
      <div className="flex items-center gap-2 border-b border-slate-800/60 px-4 py-3">
        <Sparkles className="h-4 w-4 text-violet-400" aria-hidden />
        <div className="min-w-0 flex-1">
          <div className="text-sm font-semibold text-slate-100">Ask Aether</div>
          <div className="text-[10px] uppercase tracking-[0.16em] text-slate-500">Infrastructure agent</div>
        </div>
        {messages.length > 0 ? (
          <button
            type="button"
            onClick={clearChat}
            className="rounded-lg border border-slate-700 px-2 py-1 text-[10px] text-slate-400 hover:text-slate-200"
          >
            Clear
          </button>
        ) : null}
        <button
          type="button"
          onClick={() => setCollapsed(true)}
          className="rounded-lg p-1.5 text-slate-500 hover:bg-slate-800/80 hover:text-slate-200"
          title="Collapse copilot"
          aria-label="Collapse copilot"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      </div>

      <div className="flex-1 space-y-2 overflow-y-auto p-3">
        {messages.length === 0 ? (
          <div className="space-y-3">
            <p className="text-xs leading-relaxed text-slate-500">
              Not a chatbot — an infrastructure agent. Ask about health, cost, migrations, or capacity.
            </p>
            <div className="flex flex-col gap-1.5">
              {quickPrompts.map((prompt) => (
                <button
                  key={prompt}
                  type="button"
                  onClick={() => void send(prompt)}
                  className="rounded-xl border border-slate-800/80 bg-slate-900/50 px-3 py-2 text-left text-xs text-slate-300 transition hover:border-violet-500/30 hover:text-violet-100"
                >
                  {prompt}
                </button>
              ))}
            </div>
          </div>
        ) : null}

        {messages.map((msg, i) => (
          <div
            key={`${msg.role}-${i}`}
            className={`max-w-full rounded-2xl px-3 py-2 text-xs whitespace-pre-wrap ${
              msg.role === 'user'
                ? 'ml-4 bg-violet-600/25 text-violet-50'
                : 'mr-2 bg-slate-800/80 text-slate-200'
            }`}
          >
            {msg.content}
          </div>
        ))}

        {pending.length > 0 ? (
          <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 p-2">
            <p className="mb-1 text-[10px] font-medium text-amber-200">Awaiting confirmation</p>
            {pending.map((a) => (
              <div key={a.id} className="flex items-center justify-between gap-2 py-1 text-[10px] text-slate-300">
                <span className="min-w-0 truncate">{a.description}</span>
                <button
                  type="button"
                  onClick={() => confirmAction(a.id)}
                  className="shrink-0 rounded bg-amber-600 px-2 py-0.5 text-white hover:bg-amber-500"
                >
                  Confirm
                </button>
              </div>
            ))}
          </div>
        ) : null}

        {loading ? <div className="animate-pulse text-[10px] text-slate-500">Thinking…</div> : null}
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
          placeholder="Ask Aether…"
          data-testid="copilot-rail-input"
          className="min-w-0 flex-1 rounded-xl border border-slate-700/80 bg-slate-900/60 px-3 py-2 text-xs text-slate-100 outline-none focus:border-violet-500/50"
          disabled={loading}
        />
        <button
          type="submit"
          disabled={loading || !input.trim()}
          className="rounded-xl bg-violet-600 p-2 text-white hover:bg-violet-500 disabled:opacity-50"
          aria-label="Send"
        >
          <Send className="h-4 w-4" />
        </button>
      </form>
    </aside>
  );
}

/** Mobile/tablet toggle button shown below xl breakpoint */
export function CopilotRailToggle({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="fixed bottom-6 right-6 z-30 flex items-center gap-2 rounded-full border border-violet-500/40 bg-violet-600/90 px-4 py-2.5 text-sm font-medium text-white shadow-lg backdrop-blur-xl xl:hidden"
      data-testid="copilot-rail-mobile-toggle"
    >
      <Bot className="h-4 w-4" />
      Ask Aether
      <ChevronLeft className="h-4 w-4 opacity-70" />
    </button>
  );
}
