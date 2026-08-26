// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import { DollarSign, Server, Shield, TrendingUp, Boxes } from 'lucide-react';
import { formatUSD } from '../utils/formatters';
import type { AppView } from '../types/api';
import { useFleetIntelligence } from '../hooks/useFleetIntelligence';

interface FleetIntelligenceBriefProps {
  onNavigate: (view: AppView) => void;
  refreshKey?: number;
}

export default function FleetIntelligenceBrief({ onNavigate, refreshKey = 0 }: FleetIntelligenceBriefProps) {
  const {
    loading,
    workloads,
    briefing,
    healthy,
    riskCount,
    savings,
    securityIssues,
    clusterCount,
  } = useFleetIntelligence(refreshKey);

  if (loading) {
    return (
      <div className="command-center-shell mb-6 animate-pulse p-6 sm:p-8">
        <div className="h-6 w-40 rounded glass-inset-surface" />
        <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="h-20 rounded-2xl glass-inset-surface/70" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <section className="command-center-shell mb-8 p-6 sm:p-8" data-testid="fleet-intelligence-brief">
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-brand">Fleet Intelligence</p>
          <h2 className="mt-1 text-xl font-semibold text-ink">AI-generated fleet posture</h2>
        </div>
        <button
          type="button"
          onClick={() => onNavigate('intelligence')}
          className="text-xs font-medium text-brand hover:text-blue-300 transition-colors"
        >
          Full intelligence →
        </button>
      </div>

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
        <button type="button" onClick={() => onNavigate('workloads')} className="glass-metric-card glass-metric-accent-blue text-left">
          <Boxes className="mb-2 h-4 w-4 text-ink-3" />
          <div className={`text-2xl font-semibold ${workloads.length === 0 ? 'text-ink-3' : 'text-ink'}`}>{workloads.length}</div>
          <div className="text-xs text-ink-3">Workloads</div>
        </button>
        <button type="button" onClick={() => onNavigate('fleet')} className="glass-metric-card glass-metric-accent-purple text-left">
          <Server className="mb-2 h-4 w-4 text-ink-3" />
          <div className={`text-2xl font-semibold ${clusterCount === 0 ? 'text-ink-3' : 'text-ink'}`}>{clusterCount || '—'}</div>
          <div className="text-xs text-ink-3">Clusters</div>
        </button>
        <button type="button" onClick={() => onNavigate('health')} className="glass-metric-card glass-metric-accent-emerald text-left">
          <TrendingUp className="mb-2 h-4 w-4 text-emerald-400" />
          <div className={`text-2xl font-semibold ${healthy === 0 ? 'text-ink-3' : 'text-emerald-300'}`}>{healthy}</div>
          <div className="text-xs text-ink-3">Healthy</div>
        </button>
        <button type="button" onClick={() => onNavigate('observability')} className="glass-metric-card glass-metric-accent-amber text-left">
          <TrendingUp className="mb-2 h-4 w-4 text-amber-400" />
          <div className={`text-2xl font-semibold ${riskCount === 0 ? 'text-ink-3' : 'text-amber-200'}`}>{riskCount}</div>
          <div className="text-xs text-ink-3">At risk</div>
        </button>
        <button type="button" onClick={() => onNavigate('cost')} className="glass-metric-card glass-metric-accent-emerald text-left">
          <DollarSign className="mb-2 h-4 w-4 text-emerald-400" />
          <div className={`text-2xl font-semibold ${savings === 0 ? 'text-ink-3' : 'text-emerald-300'}`}>{formatUSD(savings)}</div>
          <div className="text-xs text-ink-3">Savings/mo</div>
        </button>
        <button type="button" onClick={() => onNavigate('security')} className="glass-metric-card glass-metric-accent-amber text-left">
          <Shield className="mb-2 h-4 w-4 text-red-400" />
          <div className={`text-2xl font-semibold ${securityIssues === 0 ? 'text-ink-3' : 'text-red-300'}`}>{securityIssues}</div>
          <div className="text-xs text-ink-3">Security issues</div>
        </button>
      </div>

      {briefing?.capacity_risks[0] ? (
        <p className="mt-4 text-sm text-amber-200/90">
          Capacity: {briefing.capacity_risks[0].summary}
        </p>
      ) : null}
    </section>
  );
}
