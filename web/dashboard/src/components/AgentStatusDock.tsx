// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
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

export default function AgentStatusDock() {
  const navigate = useNavigate();
  const [agents, setAgents] = useState<AgentCard[]>([]);
  const [collapsed, setCollapsed] = useState(false);

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
      ? 'border-red-500/40 bg-red-500/10'
      : status === 'active'
        ? 'border-emerald-500/30 bg-emerald-500/5'
        : 'border-slate-800/70 bg-slate-950/80';

  return (
    <div className="fixed bottom-4 right-4 z-40 hidden lg:block" data-testid="agent-status-dock">
      <button
        type="button"
        onClick={() => setCollapsed((c) => !c)}
        className="mb-2 rounded-full border border-slate-700 bg-slate-950/90 px-3 py-1 text-[11px] font-medium uppercase tracking-wider text-slate-400 backdrop-blur hover:text-white"
      >
        {collapsed ? 'Show agents' : 'Hide agents'}
      </button>
      {!collapsed ? (
        <div className="grid max-w-sm grid-cols-2 gap-2">
          {agents.map((agent) => {
            const Icon = agent.icon;
            return (
              <button
                key={agent.id}
                type="button"
                onClick={() => navigate(viewToPath(agent.view))}
                className={`rounded-2xl border px-3 py-2 text-left backdrop-blur-xl transition hover:border-aether/40 ${tone(agent.status)}`}
              >
                <div className="flex items-center gap-2">
                  <Icon className="h-3.5 w-3.5 text-slate-400" />
                  <span className="text-xs font-medium text-white">{agent.label}</span>
                </div>
                <div className="mt-1 text-[11px] text-slate-500">{agent.detail}</div>
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
