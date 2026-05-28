// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { AlertTriangle, Brain, DollarSign, Inbox, MapPin, Sparkles, TrendingUp } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { apiFetchSettled, apiPost } from '../../utils/api';
import { formatPercent, formatUSD } from '../../utils/formatters';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import PageTabs from '../PageTabs';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
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

export default function IntelligencePage() {
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
  }, []);

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
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

      {workloadFocus ? (
        <div
          data-testid="intelligence-workload-context"
          className="mb-6 rounded-xl border border-aether/30 bg-aether/5 px-4 py-3 text-sm text-slate-300"
        >
          Intelligence context for workload <span className="font-mono text-aether">{workloadFocus}</span>
          {' · '}
          <button
            type="button"
            onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { workload: workloadFocus }))}
            className="text-aether hover:underline"
          >
            Open workload →
          </button>
        </div>
      ) : null}

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
              <div className="dash-card flex flex-wrap items-center gap-4">
                <Brain className="text-aether shrink-0" size={22} />
                <div>
                  <p className="text-sm text-slate-400">Fleet risk score</p>
                  <p className="text-2xl font-semibold text-slate-100">
                    {formatPercent(predictions.fleet_risk_score * 100, 1)}
                  </p>
                </div>
                <p className="text-xs text-slate-500 ml-auto">Generated {predictions.generated_at}</p>
              </div>
              {predictions.predictions.length === 0 ? (
                <EmptyState icon={<Inbox size={40} />} title="No predictions" description="No workloads in state store yet." />
              ) : (
                <div className="grid gap-3">
                  {predictions.predictions.map((row) => (
                    <div
                      key={row.workload}
                      className={`dash-card ${
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
                      <p className="text-xs text-slate-500 mb-3">Risk score {formatPercent(row.risk_score * 100, 1)}</p>
                      {row.predictions.length > 0 && (
                        <ul className="space-y-2 text-sm">
                          {row.predictions.map((sig, i) => (
                            <li key={`${sig.kind}-${i}`} className="rounded-lg border border-slate-800 px-3 py-2">
                              <div className="flex items-center justify-between gap-2">
                                <span className="text-slate-200">{sig.kind}</span>
                                <span className="text-xs text-slate-500">{sig.horizon}</span>
                              </div>
                              <p className="text-xs text-slate-400 mt-1">{sig.reason}</p>
                              <p className="text-xs text-aether mt-1">{formatPercent(sig.probability * 100, 0)} probability</p>
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
                className={`dash-card ${
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
                  <Link to={viewToPath('alerts')} className="text-xs text-aether hover:underline ml-auto" data-testid="intelligence-alerts-link">
                    Alerts →
                  </Link>
                </div>
                <p className="text-sm text-slate-300">{t.reason}</p>
                <p className="text-xs text-slate-500 mt-2">Score {t.score.toFixed(2)} · {t.detected_at}</p>
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
              <div className="dash-card">
                <p className="text-sm text-slate-400">Total potential savings</p>
                <p className="text-2xl font-semibold text-emerald-400">
                  {formatPercent(cost.total_potential_savings_pct, 1)}
                </p>
              </div>
              {cost.recommendations.length === 0 ? (
                <EmptyState icon={<DollarSign size={40} />} title="No cost recommendations" description="Fleet is already well-sized for current profiles." />
              ) : (
                cost.recommendations.map((rec) => (
                  <div
                    key={rec.workload}
                    className={`dash-card ${
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
                      <Badge text={`${formatPercent(rec.savings_pct, 0)} savings`} variant="green" />
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
            <Link to={viewToPath('ai')} className="text-xs text-aether hover:underline" data-testid="intelligence-ai-link">
              AI engine →
            </Link>
          </div>
          {evolution && evolution.workloads.length > 0 ? (
            evolution.workloads.map((row) => (
              <div
                key={row.workload}
                className={`dash-card ${
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
                  <Badge text={`${formatPercent(row.improvement_pct, 0)} improvement`} variant="blue" />
                </div>
                <p className="text-sm text-slate-300">
                  {row.current_runtime} → {row.recommended_runtime}
                </p>
                <p className="text-xs text-slate-500 mt-1">Confidence {formatPercent(row.confidence * 100, 0)}</p>
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
            <Link to={viewToPath('scheduler')} className="text-xs text-aether hover:underline" data-testid="intelligence-scheduler-link">
              Placement scheduler →
            </Link>
          </div>
          <div className="dash-card space-y-3">
            <p className="text-sm text-slate-400">
              POST workload YAML to rank clusters and runtimes for global placement.
            </p>
            <textarea
              value={placeYaml}
              onChange={(e) => setPlaceYaml(e.target.value)}
              rows={14}
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 font-mono text-xs text-slate-100"
            />
            <button
              type="button"
              data-testid="intelligence-place-submit"
              disabled={placeBusy}
              onClick={() => void runPlacement()}
              className="rounded-xl bg-aether px-4 py-2 text-sm font-medium text-white hover:bg-aether/90 disabled:opacity-50"
            >
              {placeBusy ? 'Ranking…' : 'Recommend placement'}
            </button>
            {placeError && <p className="text-sm text-red-400">{placeError}</p>}
          </div>
          {placeResults && placeResults.length > 0 && (
            <div className="space-y-3">
              {placeResults.map((rec, i) => (
                <div key={`${rec.cluster ?? 'local'}-${rec.runtime}-${i}`} className="dash-card">
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
    </div>
  );
}
