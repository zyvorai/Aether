// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router';
import { Bot, DollarSign, GitBranch, HeartPulse, Rocket, Shield } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { viewToPath } from '../utils/dashboardRoutes';
import type { AgentRegistryReport, AppView } from '../types/api';

interface AgentCard {
  id: string;
  label: string;
  status: 'active' | 'idle' | 'alert';
  detail: string;
  view: AppView;
  icon: typeof Bot;
}

const AGENT_ICONS: Record<string, typeof Bot> = {
  sre: HeartPulse,
  cost: DollarSign,
  security: Shield,
  capacity: Bot,
  migration: Rocket,
  gitops: GitBranch,
};

function normalizeStatus(status: string): AgentCard['status'] {
  if (status === 'alert' || status === 'active' || status === 'idle') return status;
  return 'idle';
}

function agentStatusDot(status: AgentCard['status']): string {
  if (status === 'alert') return 'bg-danger shadow-[0_0_8px_rgba(248,113,113,0.6)]';
  if (status === 'active') return 'bg-success shadow-[0_0_8px_rgba(52,211,153,0.5)]';
  return 'glass-status-dot-muted';
}

export default function AgentStatusDock() {
  const navigate = useNavigate();
  const [agents, setAgents] = useState<AgentCard[]>([]);
  const [collapsed, setCollapsed] = useState(true);
  const dockRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (collapsed) return;
    const onDocClick = (e: MouseEvent) => {
      if (dockRef.current && !dockRef.current.contains(e.target as Node)) setCollapsed(true);
    };
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, [collapsed]);

  useEffect(() => {
    let cancelled = false;
    apiFetch<AgentRegistryReport>('/intelligence/agents/status').then((report) => {
      if (cancelled || !report) return;
      const cards: AgentCard[] = report.agents.map((agent) => ({
        id: agent.id,
        label: agent.label,
        status: normalizeStatus(agent.status),
        detail: agent.detail,
        view: agent.route as AppView,
        icon: AGENT_ICONS[agent.id] ?? Bot,
      }));
      setAgents(cards);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (agents.length === 0) return null;

  const tone = (status: AgentCard['status']) =>
    status === 'alert'
      ? 'border-danger/30 bg-danger/[0.08]'
      : status === 'active'
        ? 'border-success/25 bg-success/[0.06]'
        : 'glass-divider glass';

  return (
    <div
      ref={dockRef}
      className="fixed bottom-4 right-4 z-40 hidden lg:block xl:bottom-6 xl:right-6"
      data-testid="agent-status-dock"
    >
      <button
        type="button"
        onClick={() => setCollapsed((c) => !c)}
        className="glass border border-border shadow-card mb-2 ml-auto block rounded-full px-3 py-1 text-[11px] font-medium uppercase tracking-wider text-muted transition hover:border-primary/30 hover:text-primary"
      >
        {collapsed ? 'Show agents' : 'Hide agents'}
      </button>
      {!collapsed ? (
        <div className="glass grid max-h-[60vh] max-w-sm grid-cols-2 gap-2 overflow-y-auto p-3">
          {agents.map((agent) => {
            const Icon = agent.icon;
            return (
              <button
                key={agent.id}
                type="button"
                onClick={() => navigate(viewToPath(agent.view))}
                className={`copilot-agent-chip rounded-xl px-3 py-2 text-left backdrop-blur-xl transition hover:border-primary/30 ${tone(agent.status)}`}
              >
                <div className="flex items-center gap-2">
                  <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${agentStatusDot(agent.status)}`} />
                  <Icon className="h-3.5 w-3.5 text-muted" />
                  <span className="truncate text-xs font-medium text-foreground">{agent.label}</span>
                </div>
                <div className="mt-1 truncate pl-3.5 text-[11px] text-subtle">{agent.detail}</div>
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
