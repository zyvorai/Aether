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
      <div className="glass mb-6 animate-pulse p-6 sm:p-8">
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
    <section className="glass mb-8 p-6 sm:p-8" data-testid="fleet-intelligence-brief">
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-primary">Fleet Intelligence</p>
          <h2 className="mt-1 text-xl font-semibold text-foreground">AI-generated fleet posture</h2>
        </div>
        <button
          type="button"
          onClick={() => onNavigate('intelligence')}
          className="text-xs font-medium text-primary hover:underline transition-colors"
        >
          Full intelligence →
        </button>
      </div>

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
        <button type="button" onClick={() => onNavigate('workloads')} className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4 text-left">
          <Boxes className="mb-2 h-4 w-4 text-subtle" />
          <div className={`text-2xl font-semibold ${workloads.length === 0 ? 'text-subtle' : 'text-foreground'}`}>{workloads.length}</div>
          <div className="text-xs text-subtle">Workloads</div>
        </button>
        <button type="button" onClick={() => onNavigate('fleet')} className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4 text-left">
          <Server className="mb-2 h-4 w-4 text-subtle" />
          <div className={`text-2xl font-semibold ${clusterCount === 0 ? 'text-subtle' : 'text-foreground'}`}>{clusterCount || '—'}</div>
          <div className="text-xs text-subtle">Clusters</div>
        </button>
        <button type="button" onClick={() => onNavigate('health')} className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4 text-left">
          <TrendingUp className="mb-2 h-4 w-4 text-success" />
          <div className={`text-2xl font-semibold ${healthy === 0 ? 'text-subtle' : 'text-success'}`}>{healthy}</div>
          <div className="text-xs text-subtle">Healthy</div>
        </button>
        <button type="button" onClick={() => onNavigate('observability')} className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4 text-left">
          <TrendingUp className="mb-2 h-4 w-4 text-warning" />
          <div className={`text-2xl font-semibold ${riskCount === 0 ? 'text-subtle' : 'text-warning'}`}>{riskCount}</div>
          <div className="text-xs text-subtle">At risk</div>
        </button>
        <button type="button" onClick={() => onNavigate('cost')} className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4 text-left">
          <DollarSign className="mb-2 h-4 w-4 text-success" />
          <div className={`text-2xl font-semibold ${savings === 0 ? 'text-subtle' : 'text-success'}`}>{formatUSD(savings)}</div>
          <div className="text-xs text-subtle">Savings/mo</div>
        </button>
        <button type="button" onClick={() => onNavigate('security')} className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4 text-left">
          <Shield className="mb-2 h-4 w-4 text-danger" />
          <div className={`text-2xl font-semibold ${securityIssues === 0 ? 'text-subtle' : 'text-danger'}`}>{securityIssues}</div>
          <div className="text-xs text-subtle">Security issues</div>
        </button>
      </div>

      {briefing?.capacity_risks[0] ? (
        <p className="mt-4 text-sm text-warning/90">
          Capacity: {briefing.capacity_risks[0].summary}
        </p>
      ) : null}
    </section>
  );
}
