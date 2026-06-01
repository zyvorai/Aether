// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { AlertTriangle, Loader2, RefreshCw, Stethoscope } from 'lucide-react';
import { apiFetch } from '../utils/api';
import { formatPercent } from '../utils/formatters';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import Badge from './Badge';
import GlassSection from './GlassSection';

import type { FleetRootCauseReport } from '../types/api';

function confidenceVariant(c: number): 'green' | 'yellow' | 'red' | 'muted' {
  if (c >= 0.85) return 'green';
  if (c >= 0.7) return 'yellow';
  return 'red';
}

export default function FleetRootCausePanel() {
  const navigate = useNavigate();
  const [report, setReport] = useState<FleetRootCauseReport | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<FleetRootCauseReport>('/copilot/troubleshoot/fleet');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <GlassSection
      accent="blue"
      testId="fleet-root-cause-panel"
      label="AI Root Cause Analysis"
      title="Fleet incident correlation"
      subtitle="Batch diagnosis across unhealthy workloads with evidence and recommendations."
      icon={<Stethoscope className="h-5 w-5 text-violet-300" />}
      actions={
        <button
          type="button"
          onClick={() => void load()}
          className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-sm text-slate-300 hover:border-aether/40"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
          Scan fleet
        </button>
      }
    >
      {loading && !report ? (
        <div className="flex items-center gap-2 py-8 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" />
          Correlating logs, events, and health signals…
        </div>
      ) : null}

      {!loading && report && report.diagnoses.length === 0 ? (
        <div className="rounded-2xl border border-emerald-500/20 bg-emerald-500/5 px-4 py-6 text-center text-sm text-emerald-200">
          No unhealthy workloads detected in the last fleet scan ({report.scanned} scanned).
        </div>
      ) : null}

      {report && report.diagnoses.length > 0 ? (
        <div className="space-y-3">
          {report.diagnoses.map((row) => (
            <article
              key={row.workload}
              className="glass-panel-card p-4"
              data-testid={`root-cause-${row.workload}`}
            >
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <button
                    type="button"
                    onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: row.workload }))}
                    className="text-left text-base font-semibold text-white hover:text-aether"
                  >
                    {row.workload}
                  </button>
                  <p className="mt-1 text-sm text-slate-400">{row.summary}</p>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Badge text={row.health_level} variant={row.health_level === 'failing' ? 'red' : 'yellow'} />
                  <Badge text={`${formatPercent(row.confidence, 0)} confidence`} variant={confidenceVariant(row.confidence)} />
                </div>
              </div>

              <div className="mt-4 grid gap-4 lg:grid-cols-2">
                <div>
                  <div className="text-[10px] uppercase tracking-wider text-slate-500">Likely cause</div>
                  <div className="mt-1 flex items-center gap-2 text-sm font-medium text-amber-100">
                    <AlertTriangle className="h-4 w-4 shrink-0 text-amber-400" />
                    {row.likely_cause}
                  </div>
                </div>
                <div>
                  <div className="text-[10px] uppercase tracking-wider text-slate-500">Recommendation</div>
                  <p className="mt-1 text-sm text-slate-300">{row.recommendation}</p>
                </div>
              </div>

              {row.evidence.length > 0 ? (
                <ul className="mt-3 space-y-1 glass-divider-t/60 pt-3 text-xs text-slate-500">
                  {row.evidence.slice(0, 4).map((ev) => (
                    <li key={ev}>• {ev}</li>
                  ))}
                </ul>
              ) : null}

              <div className="mt-3 flex flex-wrap gap-3 text-xs">
                <Link to={pathWithQuery(viewToPath('workloads'), { workload: row.workload, tab: 'logs' })} className="text-aether hover:underline">
                  View logs →
                </Link>
                <Link to={pathWithQuery(viewToPath('copilot'), { workload: row.workload, q: `Fix ${row.workload}` })} className="text-aether hover:underline">
                  Ask copilot →
                </Link>
              </div>
            </article>
          ))}
        </div>
      ) : null}
    </GlassSection>
  );
}
