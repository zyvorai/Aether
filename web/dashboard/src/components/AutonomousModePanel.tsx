// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Bot, Loader2, RefreshCw, ShieldCheck } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { viewToPath } from '../utils/dashboardRoutes';
import Badge from './Badge';
import type { AutonomyStatusReport } from '../types/api';
import GlassSection from './GlassSection';

function tierVariant(enabled: boolean): 'green' | 'muted' {
  return enabled ? 'green' : 'muted';
}

export default function AutonomousModePanel() {
  const [report, setReport] = useState<AutonomyStatusReport | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<AutonomyStatusReport>('/intelligence/autonomy/status');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const policy = report?.effective_policy;

  return (
    <GlassSection
      accent="purple"
      testId="autonomous-mode-panel"
      title="Autonomous Mode"
      subtitle="Global policy for self-healing, drift reconcile, migration, and evolution agents."
      icon={<Bot className="h-5 w-5 text-lavender" />}
      actions={
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-muted hover:border-primary/40"
        >
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          Refresh
        </button>
      }
    >
      {report ? (
        <>
          <div className="mb-6 flex flex-wrap items-center gap-3">
            <Badge
              text={report.autonomy_enabled ? 'Agents armed' : 'Recommend only'}
              variant={report.autonomy_enabled ? 'green' : 'yellow'}
            />
            <span className="text-xs text-subtle">
              AETHER_AUTO_RESTART={report.env.aether_auto_restart ? '1' : '0'} · AETHER_AUTO_RECONCILE=
              {report.env.aether_auto_reconcile ? '1' : '0'}
            </span>
          </div>

          <div className="mb-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
              <ShieldCheck className="mb-2 h-4 w-4 text-success" />
              <Badge text={policy?.auto_restart ? 'On' : 'Off'} variant={tierVariant(!!policy?.auto_restart)} />
              <div className="mt-2 text-xs text-subtle">Auto restart</div>
            </div>
            <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
              <Badge
                text={policy?.auto_reconcile_drift ? 'On' : 'Off'}
                variant={tierVariant(!!policy?.auto_reconcile_drift)}
              />
              <div className="mt-2 text-xs text-subtle">Drift reconcile</div>
            </div>
            <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
              <div className="text-sm font-medium text-foreground">{policy?.auto_migrate ?? 'recommend'}</div>
              <div className="mt-2 text-xs text-subtle">Migration agent</div>
            </div>
            <div className="tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4">
              <div className="text-sm font-medium text-foreground">{policy?.auto_evolve ?? 'recommend'}</div>
              <div className="mt-2 text-xs text-subtle">Evolution agent</div>
            </div>
          </div>

          {report.workload_overrides.length > 0 ? (
            <div className="mb-6">
              <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-subtle">Workload overrides</p>
              <ul className="space-y-2">
                {report.workload_overrides.map((row) => (
                  <li key={row.workload} className="rounded-xl border glass-divider px-3 py-2 text-sm text-muted">
                    <Link
                      to={`${viewToPath('workloads')}?workload=${encodeURIComponent(row.workload)}`}
                      className="font-medium text-primary hover:underline"
                    >
                      {row.workload}
                    </Link>
                    <span className="text-subtle">
                      {' '}
                      · heal {row.healing} · migrate {row.migration} · evolve {row.evolution}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}

          <ul className="space-y-2">
            {report.recommendations.map((line) => (
              <li key={line} className="rounded-lg border glass-divider px-3 py-2 text-sm text-muted">
                {line}
              </li>
            ))}
          </ul>
        </>
      ) : loading ? (
        <div className="flex items-center gap-2 text-sm text-subtle">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading autonomy policy…
        </div>
      ) : null}
    </GlassSection>
  );
}
