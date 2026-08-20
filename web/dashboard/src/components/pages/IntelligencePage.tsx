// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { AlertTriangle, Brain, DollarSign, Inbox, MapPin, Sparkles, TrendingUp } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatPercent, formatTimestamp, formatUSD } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import PageTabs from '../PageTabs';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import GlassSection from '../GlassSection';
import StatCard from '../StatCard';
import type {
  CostOptimizeReport,
  EvolutionStatus,
  PlacementRecommendation,
  PredictionReport,
  ThreatReport,
} from '../../types/api';

const PLACE_SAMPLE = `apiVersion: aether/v1
kind: Workload
metadata:
  name: placement-demo
  owner: dashboard
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 500m
  memory: 512Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow:
    - kube
`;

type IntelTab = 'predictions' | 'threats' | 'cost' | 'evolution' | 'place';

function riskVariant(level: string): 'green' | 'yellow' | 'red' | 'muted' {
  const l = level.toLowerCase();
  if (l === 'low') return 'green';
  if (l === 'medium' || l === 'moderate') return 'yellow';
  if (l === 'high' || l === 'critical') return 'red';
  return 'muted';
}

function workloadMatchesFocus(rowWorkload: string, focus: string): boolean {
  const needle = focus.trim();
  if (!needle) return false;
  if (rowWorkload === needle) return true;
  return rowWorkload.endsWith(`/${needle}`) || rowWorkload.split('/').includes(needle);
}

export default function IntelligencePage({ refreshKey }: { refreshKey?: number } = {}) {
  const navigate = useNavigate();
  const [tabParam, setTabParam] = useQueryParam('tab', 'predictions');
  const [workloadQuery] = useQueryParam('workload', '');
  const workloadFocus = workloadQuery.trim() || undefined;
  const tab: IntelTab = (
    ['predictions', 'threats', 'cost', 'evolution', 'place'] as const
  ).includes(tabParam as IntelTab)
    ? (tabParam as IntelTab)
    : 'predictions';
  const setTab = (next: IntelTab) => setTabParam(next);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [predictions, setPredictions] = useState<PredictionReport | null>(null);
  const [threats, setThreats] = useState<ThreatReport | null>(null);
  const [cost, setCost] = useState<CostOptimizeReport | null>(null);
  const [evolution, setEvolution] = useState<EvolutionStatus | null>(null);
  const [placeYaml, setPlaceYaml] = useState(PLACE_SAMPLE);
  const [placeResults, setPlaceResults] = useState<PlacementRecommendation[] | null>(null);
  const [placeBusy, setPlaceBusy] = useState(false);
  const [placeError, setPlaceError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const [pred, thr, fin, evo] = await Promise.all([
      apiFetchSettled<PredictionReport>('/intelligence/predictions'),
      apiFetchSettled<ThreatReport>('/intelligence/threats'),
      apiFetchSettled<CostOptimizeReport>('/intelligence/cost-optimize'),
      apiFetchSettled<EvolutionStatus>('/intelligence/evolution/status'),
    ]);
    if (!pred.ok && !thr.ok && !fin.ok && !evo.ok) {
      setLoadFailed(true);
    } else {
      setPredictions(pred.ok ? pred.data : null);
      setThreats(thr.ok ? thr.data : null);
      setCost(fin.ok ? fin.data : null);
      setEvolution(evo.ok ? evo.data : null);
    }
    setLoading(false);
  }, [refreshKey]);

  useEffect(() => {
    void load();
  }, [load]);

  const focusTab = useMemo((): IntelTab | null => {
    if (!workloadFocus) return null;
    if (predictions?.predictions.some((row) => workloadMatchesFocus(row.workload, workloadFocus))) {
      return 'predictions';
    }
    if (threats?.threats.some((t) => workloadMatchesFocus(t.workload, workloadFocus))) return 'threats';
    if (cost?.recommendations.some((rec) => workloadMatchesFocus(rec.workload, workloadFocus))) return 'cost';
    if (evolution?.workloads.some((row) => workloadMatchesFocus(row.workload, workloadFocus))) {
      return 'evolution';
    }
    return null;
  }, [workloadFocus, predictions, threats, cost, evolution]);

  useEffect(() => {
    if (!workloadFocus || !focusTab) return;
    setTab(focusTab);
  }, [workloadFocus, focusTab, setTab]);

  async function runPlacement() {
    setPlaceBusy(true);
    setPlaceError(null);
    const res = await apiPost<PlacementRecommendation[]>('/intelligence/place', { yaml: placeYaml });
    setPlaceBusy(false);
    if (res.success && res.data) {
      setPlaceResults(res.data);
    } else {
      setPlaceResults(null);
      setPlaceError(res.error ?? 'Placement request failed');
    }
  }

  if (loading && !predictions && !threats && !cost && !evolution && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Intelligence data unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <div className="mb-6 glass-context-banner" data-testid="intelligence-hub-context">
        Intelligence
        {' · '}
        <Link to={viewToPath('health')} className="text-aether hover:underline" data-testid="intelligence-context-orchestrator-link">
          Orchestrator →
        </Link>
        {' · '}
        <Link to={viewToPath('fleet')} className="text-aether hover:underline" data-testid="intelligence-context-fleet-link">
          Fleet →
        </Link>
        {' · '}
        <Link
          to={`${viewToPath('fleet')}?tab=edge`}
          className="text-aether hover:underline"
          data-testid="intelligence-context-edge-link"
        >
          Edge →
        </Link>
        {' · '}
        <Link to={viewToPath('hosted')} className="text-aether hover:underline" data-testid="intelligence-context-hosted-link">
          Hosted SaaS →
        </Link>
      </div>
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

      {workloadFocus ? (
        <WorkloadContextBanner
          testId="intelligence-workload-context"
          workload={workloadFocus}
          description="Intelligence context"
        >
          <WorkloadScopedCrossLinks
            workload={workloadFocus}
            prefix="intelligence"
            showDrift
            showGitops
            showMetrics
          />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('ai'), { workload: workloadFocus, tab: 'analyze' })}
            className="text-aether hover:underline"
            data-testid="intelligence-ai-link"
          >
            AI engine →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('zyra'), { workload: workloadFocus, q: `Risk summary for ${workloadFocus}` })}
            className="text-aether hover:underline"
            data-testid="intelligence-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('cost'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="intelligence-context-cost-link"
          >
            Cost →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="intelligence-context-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('gitops'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="intelligence-context-gitops-link"
          >
            GitOps →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('drift'), { workload: workloadFocus })}
            className="text-aether hover:underline"
            data-testid="intelligence-context-drift-link"
          >
            Drift →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      <section className="overview-section-shell mb-6 p-6 sm:p-8">
      <GlassSection
        variant="hero"
        accent="purple"
        label="Intelligence"
        title="Predictive ops"
        subtitle="Fleet risk, threats, cost optimization, and placement intelligence"
        className="mb-6 p-6 sm:p-8"
        icon={<Brain className="h-5 w-5 text-aether-ai" />}
      >
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <StatCard
            title="Fleet risk"
            value={predictions ? formatPercent(predictions.fleet_risk_score, 1) : '—'}
            color="purple"
            compact
            isEmpty={!predictions}
          />
          <StatCard
            title="Threats"
            value={threats?.threats.length ?? 0}
            color="red"
            compact
            isEmpty={(threats?.threats.length ?? 0) === 0}
          />
          <StatCard
            title="Cost saves"
            value={cost ? `${cost.total_potential_savings_pct.toFixed(1)}%` : '—'}
            color="green"
            compact
            isEmpty={!cost}
          />
          <StatCard
            title="Evolution"
            value={evolution?.workloads.length ?? 0}
            color="blue"
            compact
            isEmpty={(evolution?.workloads.length ?? 0) === 0}
          />
        </div>
      </GlassSection>

      <div data-testid="intelligence-tabs">
      <PageTabs
        tabs={[
          { id: 'predictions', label: 'Predictions', icon: <TrendingUp size={14} /> },
          { id: 'threats', label: 'Threats', icon: <AlertTriangle size={14} /> },
          { id: 'cost', label: 'Cost optimize', icon: <DollarSign size={14} /> },
          { id: 'evolution', label: 'Evolution', icon: <Sparkles size={14} /> },
          { id: 'place', label: 'Placement', icon: <MapPin size={14} /> },
        ]}
        active={tab}
        onChange={(id) => setTab(id as IntelTab)}
      />
      </div>

      {tab === 'predictions' && (
        <div className="space-y-4 mt-4" data-testid="intelligence-predictions-panel">
          {predictions ? (
            <>
              <div className="glass-panel-card flex flex-wrap items-center gap-4">
                <Brain className="text-aether shrink-0" size={22} />
                <div>
                  <p className="text-sm text-slate-400">Fleet risk score</p>
                  <p className="text-2xl font-semibold text-slate-100">
                    {formatPercent(predictions.fleet_risk_score, 1)}
                  </p>
                </div>
                <p className="text-xs text-slate-500 ml-auto">Generated {formatTimestamp(predictions.generated_at)}</p>
              </div>
              {predictions.predictions.length === 0 ? (
                <EmptyState icon={<Inbox size={40} />} title="No predictions" description="No workloads in state store yet." />
              ) : (
                <div className="grid gap-3">
                  {predictions.predictions.map((row) => (
                    <div
                      key={row.workload}
                      className={`glass-panel-card ${
                        workloadFocus && workloadMatchesFocus(row.workload, workloadFocus)
                          ? 'ring-1 ring-aether/40 border-aether/30'
                          : ''
                      }`}
                      data-testid={
                        workloadFocus && workloadMatchesFocus(row.workload, workloadFocus)
                          ? 'intelligence-workload-highlight'
                          : undefined
                      }
                    >
                      <div className="flex items-center justify-between gap-2 mb-2">
                        <Link
                          to={pathWithQuery(viewToPath('workloads'), { workload: row.workload })}
                          className="font-medium text-slate-100 hover:text-aether"
                        >
                          {row.workload}
                        </Link>
                        <Link
                          to={pathWithQuery(viewToPath('events'), { workload: row.workload })}
                          className="text-xs text-slate-500 hover:text-aether"
                        >
                          Events →
                        </Link>
                        <Badge text={row.risk_level} variant={riskVariant(row.risk_level)} />
                      </div>
                      <p className="text-xs text-slate-500 mb-3">Risk score {formatPercent(row.risk_score, 1)}</p>
                      {row.predictions.length > 0 && (
                        <ul className="space-y-2 text-sm">
                          {row.predictions.map((sig, i) => (
                            <li key={`${sig.kind}-${i}`} className="rounded-lg border glass-divider px-3 py-2">
                              <div className="flex items-center justify-between gap-2">
                                <span className="text-slate-200">{sig.kind}</span>
                                <span className="text-xs text-slate-500">{sig.horizon}</span>
                              </div>
                              <p className="text-xs text-slate-400 mt-1">{sig.reason}</p>
                              <p className="text-xs text-aether mt-1">{formatPercent(sig.probability, 0)} probability</p>
                            </li>
                          ))}
                        </ul>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </>
          ) : (
            <p className="text-sm text-slate-500">Predictions unavailable.</p>
          )}
        </div>
      )}

      {tab === 'threats' && (
        <div className="space-y-4 mt-4" data-testid="intelligence-threats-panel">
          {threats && threats.threats.length > 0 ? (
            threats.threats.map((t) => (
              <div
                key={`${t.workload}-${t.detected_at}`}
                className={`glass-panel-card ${
                  workloadFocus && workloadMatchesFocus(t.workload, workloadFocus)
                    ? 'ring-1 ring-aether/40 border-aether/30'
                    : ''
                }`}
                data-testid={
                  workloadFocus && workloadMatchesFocus(t.workload, workloadFocus)
                    ? 'intelligence-workload-highlight'
                    : undefined
                }
              >
                <div className="flex flex-wrap items-center gap-2 mb-2">
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: t.workload })}
                    className="font-medium text-slate-100 hover:text-aether"
                  >
                    {t.workload}
                  </Link>
                  <Badge text={t.severity} variant={riskVariant(t.severity)} />
                  <Badge text={t.category} variant="muted" />
                  <Link
                    to={
                      workloadFocus
                        ? pathWithQuery(viewToPath('alerts'), { workload: t.workload })
                        : viewToPath('alerts')
                    }
                    className="text-xs text-aether hover:underline ml-auto"
                    data-testid="intelligence-alerts-link"
                  >
                    Alerts →
                  </Link>
                </div>
                <p className="text-sm text-slate-300">{t.reason}</p>
                <p className="text-xs text-slate-500 mt-2">Score {t.score.toFixed(2)} · {formatTimestamp(t.detected_at)}</p>
              </div>
            ))
          ) : (
            <EmptyState icon={<AlertTriangle size={40} />} title="No threats detected" description="Security scan found no active threat signals." />
          )}
        </div>
      )}

      {tab === 'cost' && (
        <div className="space-y-4 mt-4">
          {cost ? (
            <>
              <div className="glass-panel-card">
                <p className="text-sm text-slate-400">Total potential savings</p>
                <p className="text-2xl font-semibold text-emerald-400">
                  {cost.total_potential_savings_pct.toFixed(1)}%
                </p>
              </div>
              {cost.recommendations.length === 0 ? (
                <EmptyState icon={<DollarSign size={40} />} title="No cost recommendations" description="Fleet is already well-sized for current profiles." />
              ) : (
                cost.recommendations.map((rec) => (
                  <div
                    key={rec.workload}
                    className={`glass-panel-card ${
                      workloadFocus && workloadMatchesFocus(rec.workload, workloadFocus)
                        ? 'ring-1 ring-aether/40 border-aether/30'
                        : ''
                    }`}
                    data-testid={
                      workloadFocus && workloadMatchesFocus(rec.workload, workloadFocus)
                        ? 'intelligence-workload-highlight'
                        : undefined
                    }
                  >
                    <div className="flex flex-wrap items-center gap-2 mb-2">
                      <Link
                        to={pathWithQuery(viewToPath('workloads'), { workload: rec.workload })}
                        className="font-medium text-slate-100 hover:text-aether"
                      >
                        {rec.workload}
                      </Link>
                      <Badge text={`${rec.savings_pct.toFixed(0)}% savings`} variant="green" />
                      <Badge text={rec.risk} variant={riskVariant(rec.risk)} />
                    </div>
                    <p className="text-sm text-slate-300">
                      {rec.current_runtime} → {rec.suggested_runtime}
                    </p>
                    <p className="text-xs text-slate-400 mt-1">{rec.reason}</p>
                    <p className="text-xs text-emerald-400 mt-2">{formatUSD(rec.savings_monthly_usd)}/mo estimated</p>
                  </div>
                ))
              )}
            </>
          ) : (
            <p className="text-sm text-slate-500">Cost optimization unavailable.</p>
          )}
        </div>
      )}

      {tab === 'evolution' && (
        <div className="space-y-4 mt-4" data-testid="intelligence-evolution-panel">
          <div className="flex justify-end">
            <Link
              to={
                workloadFocus
                  ? pathWithQuery(viewToPath('ai'), { workload: workloadFocus, tab: 'analyze' })
                  : viewToPath('ai')
              }
              className="text-xs text-aether hover:underline"
              data-testid="intelligence-tab-ai-link"
            >
              AI engine →
            </Link>
          </div>
          {evolution && evolution.workloads.length > 0 ? (
            evolution.workloads.map((row) => (
              <div
                key={row.workload}
                className={`glass-panel-card ${
                  workloadFocus && workloadMatchesFocus(row.workload, workloadFocus)
                    ? 'ring-1 ring-aether/40 border-aether/30'
                    : ''
                }`}
                data-testid={
                  workloadFocus && workloadMatchesFocus(row.workload, workloadFocus)
                    ? 'intelligence-workload-highlight'
                    : undefined
                }
              >
                <div className="flex flex-wrap items-center gap-2 mb-2">
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: row.workload })}
                    className="font-medium text-slate-100 hover:text-aether"
                  >
                    {row.workload}
                  </Link>
                  {row.auto_eligible && <Badge text="auto-eligible" variant="green" />}
                  <Badge text={`${row.improvement_pct.toFixed(0)}% improvement`} variant="blue" />
                </div>
                <p className="text-sm text-slate-300">
                  {row.current_runtime} → {row.recommended_runtime}
                </p>
                <p className="text-xs text-slate-500 mt-1">Confidence {formatPercent(row.confidence, 0)}</p>
                {row.reasons.length > 0 && (
                  <ul className="mt-2 space-y-1 text-xs text-slate-400">
                    {row.reasons.map((r) => (
                      <li key={r}>• {r}</li>
                    ))}
                  </ul>
                )}
              </div>
            ))
          ) : (
            <EmptyState icon={<Sparkles size={40} />} title="No evolution candidates" description="Runtime evolution has no actionable workloads right now." />
          )}
        </div>
      )}

      {tab === 'place' && (
        <div className="space-y-4 mt-4" data-testid="intelligence-place-panel">
          <div className="flex justify-end mb-2">
            <Link
              to={
                workloadFocus
                  ? pathWithQuery(viewToPath('scheduler'), { workload: workloadFocus })
                  : viewToPath('scheduler')
              }
              className="text-xs text-aether hover:underline"
              data-testid="intelligence-tab-scheduler-link"
            >
              Placement scheduler →
            </Link>
          </div>
          <div className="glass-panel-card space-y-3">
            <p className="text-sm text-slate-400">
              POST workload YAML to rank clusters and runtimes for global placement.
            </p>
            <textarea
              value={placeYaml}
              onChange={(e) => setPlaceYaml(e.target.value)}
              rows={14}
              className="glass-input font-mono text-xs"
            />
            <button
              type="button"
              data-testid="intelligence-place-submit"
              disabled={placeBusy}
              onClick={() => void runPlacement()}
              className="btn-primary disabled:opacity-50"
            >
              {placeBusy ? 'Ranking…' : 'Recommend placement'}
            </button>
            {placeError && <p className="text-sm text-red-400">{placeError}</p>}
          </div>
          {placeResults && placeResults.length > 0 && (
            <div className="space-y-3">
              {placeResults.map((rec, i) => (
                <div key={`${rec.cluster ?? 'local'}-${rec.runtime}-${i}`} className="glass-panel-card">
                  <div className="flex flex-wrap items-center gap-2 mb-2">
                    <span className="font-medium text-slate-100">{rec.cluster ?? 'default cluster'}</span>
                    <Badge text={rec.runtime} variant="blue" />
                    <Badge text={`score ${rec.score.toFixed(2)}`} variant={i === 0 ? 'green' : 'muted'} />
                    {rec.cluster && (
                      <Link
                        to={pathWithQuery(viewToPath('fleet'), { cluster: rec.cluster })}
                        className="text-xs text-aether hover:underline ml-auto"
                      >
                        Fleet →
                      </Link>
                    )}
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-xs text-slate-400 mb-2">
                    <span>Latency {rec.latency_score.toFixed(2)}</span>
                    <span>Cost {rec.cost_score.toFixed(2)}</span>
                    <span>GPU {rec.gpu_available ? 'available' : 'none'}</span>
                  </div>
                  {rec.reasons.length > 0 && (
                    <ul className="text-xs text-slate-500 space-y-1">
                      {rec.reasons.map((r) => (
                        <li key={r}>• {r}</li>
                      ))}
                    </ul>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      </section>
    </div>
  );
}
