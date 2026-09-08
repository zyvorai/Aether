// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { Link, useNavigate } from 'react-router';
import IntentStudioPanel from '../IntentStudioPanel';
import IntentPipelinePanel from '../IntentPipelinePanel';
import IntentPlatformPanel from '../IntentPlatformPanel';
import RuntimeAdvisorPanel from '../RuntimeAdvisorPanel';
import WorkloadDesignerPanel from '../WorkloadDesignerPanel';
import { Brain, Cpu, Search, Sparkles, Target, TrendingUp, Wand2, Zap } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { apiFetch, apiFetchSettled, apiPost } from '../../utils/api';
import { formatPercent, formatUSD } from '../../utils/formatters';
import YamlInput from '../YamlInput';
import BarChart from '../BarChart';
import Badge, { RuntimeBadge } from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import PageTabs from '../PageTabs';
import { SectionHeader } from '../layout/SectionHeader';
import WorkloadSelect from '../WorkloadSelect';
import SectionHubPage from '../SectionHubPage';
import type { WorkloadResponse, ScoringResult, ScalingAdvice, RuntimeScore } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function DetailRow({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4 py-2 glass-divider-b last:border-0">
      <span className="text-xs font-medium uppercase tracking-wider text-subtle shrink-0">{label}</span>
      <span className="text-sm text-foreground text-right">{value}</span>
    </div>
  );
}

function confidenceVariant(confidence: number): 'green' | 'yellow' | 'muted' {
  if (confidence >= 0.8) return 'green';
  if (confidence >= 0.6) return 'yellow';
  return 'muted';
}

function ScoringResultPanel({ result }: { result: ScoringResult }) {
  const sorted = [...result.scores].sort((a, b) => b.total_score - a.total_score);

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-center gap-2">
        <RuntimeBadge runtime={result.recommended} />
        <Badge
          text={`${formatPercent(result.confidence, 0)} confidence`}
          variant={confidenceVariant(result.confidence)}
        />
        <Badge text={result.workload_class} variant="muted" />
      </div>

      <div className="space-y-4">
        {sorted.map((s) => (
          <RuntimeScoreBlock key={s.runtime} score={s} recommended={result.recommended} />
        ))}
      </div>
    </div>
  );
}

function RuntimeScoreBlock({ score, recommended }: { score: RuntimeScore; recommended: string }) {
  const isRecommended = score.runtime === recommended;

  return (
    <div className={`rounded-xl border p-3 ${isRecommended ? 'border-primary/40 bg-primary/5' : 'glass-divider glass'}`}>
      <div className="flex items-center justify-between gap-2 mb-3">
        <RuntimeBadge runtime={score.runtime} />
        {isRecommended && <Badge text="Recommended" variant="accent" />}
      </div>
      <BarChart label="Overall score" percent={score.total_score * 100} />
      <div className="mt-3 grid grid-cols-2 gap-2">
        <BarChart label="Cost" percent={score.cost_score * 100} />
        <BarChart label="Performance" percent={score.performance_score * 100} />
        <BarChart label="Reliability" percent={score.reliability_score * 100} />
        <BarChart label="Availability" percent={score.availability_score * 100} />
      </div>
      {score.reasons.length > 0 && (
        <ul className="mt-3 space-y-1 text-xs text-muted">
          {score.reasons.map((r, i) => (
            <li key={i} className="flex gap-2">
              <span className="text-success shrink-0">+</span>
              <span>{r}</span>
            </li>
          ))}
        </ul>
      )}
      {score.warnings.length > 0 && (
        <ul className="mt-2 space-y-1 text-xs text-warning/90">
          {score.warnings.map((w, i) => (
            <li key={i} className="flex gap-2">
              <span className="shrink-0">!</span>
              <span>{w}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ScalingAdvicePanel({ advice }: { advice: ScalingAdvice }) {
  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <Badge text={advice.action} variant="blue" />
        <Badge
          text={`${formatPercent(advice.confidence, 0)} confidence`}
          variant={confidenceVariant(advice.confidence)}
        />
      </div>
      <dl className="space-y-0">
        <DetailRow label="Current replicas" value={advice.current_replicas} />
        <DetailRow label="Recommended replicas" value={advice.recommended_replicas} />
        <DetailRow label="Reason" value={advice.reason} />
      </dl>
      {advice.forecast && (
        <div className="rounded-xl border glass-divider glass p-3">
          <h4 className="text-xs font-medium uppercase tracking-wider text-subtle mb-2">Forecast</h4>
          <dl className="space-y-0">
            <DetailRow label="Trend" value={advice.forecast.trend} />
            <DetailRow label="Predicted" value={advice.forecast.predicted_value.toFixed(2)} />
            <DetailRow
              label="Range"
              value={`${advice.forecast.lower_bound.toFixed(2)} – ${advice.forecast.upper_bound.toFixed(2)}`}
            />
            <DetailRow label="Horizon" value={`${advice.forecast.horizon_minutes} min`} />
          </dl>
        </div>
      )}
      {advice.cost_impact && (
        <div className="rounded-xl border glass-divider glass p-3">
          <h4 className="text-xs font-medium uppercase tracking-wider text-subtle mb-2">Cost impact</h4>
          <dl className="space-y-0">
            <DetailRow label="Current hourly" value={formatUSD(advice.cost_impact.current_hourly)} />
            <DetailRow label="Projected hourly" value={formatUSD(advice.cost_impact.projected_hourly)} />
            <DetailRow label="Delta hourly" value={formatUSD(advice.cost_impact.delta_hourly)} />
            <DetailRow label="Delta monthly" value={formatUSD(advice.cost_impact.delta_monthly)} />
          </dl>
        </div>
      )}
    </div>
  );
}

interface WorkloadProfileResult {
  name: string;
  classification: string;
  optimization_score: number;
  resource_analysis?: {
    overall_efficiency: number;
    waste_detected: boolean;
    cpu_efficiency: number;
    memory_efficiency: number;
  };
  recommendations?: Array<{
    title: string;
    description: string;
    priority: string;
    category: string;
    estimated_savings_pct: number;
  }>;
}

interface LogAnalysisResult {
  total_lines: number;
  error_count: number;
  warning_count: number;
  error_rate: number;
  health_assessment?: { status: string; score: number; issues?: string[]; suggestions?: string[] };
  patterns?: Array<{ pattern: string; count: number; severity: string }>;
  anomalies?: Array<{ description: string; severity: string }>;
}

function ProfileResultPanel({ data }: { data: WorkloadProfileResult }) {
  const ra = data.resource_analysis;
  const scoringHref = pathWithQuery(viewToPath('workloads'), { workload: data.name, tab: 'scoring' });
  return (
    <div className="space-y-4" data-testid="ai-profile-result">
      <dl className="space-y-0">
        <DetailRow label="Workload" value={data.name} />
        <DetailRow label="Classification" value={String(data.classification).replace(/([A-Z])/g, ' $1').trim()} />
        <DetailRow label="Optimization score" value={`${data.optimization_score.toFixed(0)}%`} />
        {ra && (
          <>
            <DetailRow label="Overall efficiency" value={formatPercent(ra.overall_efficiency, 0)} />
            <DetailRow label="Waste detected" value={ra.waste_detected ? 'Yes' : 'No'} />
            <DetailRow label="CPU efficiency" value={formatPercent(ra.cpu_efficiency, 0)} />
            <DetailRow label="Memory efficiency" value={formatPercent(ra.memory_efficiency, 0)} />
          </>
        )}
      </dl>
      {data.recommendations && data.recommendations.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-xs font-medium uppercase tracking-wider text-subtle">Recommendations</h4>
          {data.recommendations.map((rec, i) => (
            <div key={i} className="rounded-lg border glass-divider glass p-3 text-sm">
              <div className="flex flex-wrap items-center gap-2 mb-1">
                <Badge text={rec.priority} variant={rec.priority === 'Critical' ? 'red' : 'blue'} />
                <span className="font-medium text-foreground">{rec.title}</span>
              </div>
              <p className="text-muted text-xs">{rec.description}</p>
              <p className="text-success text-xs mt-1">Est. savings: {rec.estimated_savings_pct.toFixed(0)}%</p>
            </div>
          ))}
        </div>
      )}
      <Link to={scoringHref} className="inline-flex text-xs text-primary hover:underline">
        Open scoring for {data.name} →
      </Link>
    </div>
  );
}

function AnalyzeResultPanel({ data, workloadName }: { data: LogAnalysisResult; workloadName: string }) {
  return (
    <div className="space-y-4">
      <dl className="space-y-0">
        <DetailRow label="Total lines" value={data.total_lines} />
        <DetailRow label="Errors" value={data.error_count} />
        <DetailRow label="Warnings" value={data.warning_count} />
        <DetailRow label="Error rate" value={formatPercent(data.error_rate, 1)} />
        {data.health_assessment && (
          <>
            <DetailRow label="Health" value={String(data.health_assessment.status)} />
            <DetailRow label="Health score" value={`${data.health_assessment.score.toFixed(0)}%`} />
          </>
        )}
      </dl>
      {data.patterns && data.patterns.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-xs font-medium uppercase tracking-wider text-subtle">Top patterns</h4>
          {data.patterns.slice(0, 5).map((p, i) => (
            <DetailRow key={i} label={`${p.severity} (${p.count})`} value={<span className="font-mono text-xs">{p.pattern}</span>} />
          ))}
        </div>
      )}
      {data.anomalies && data.anomalies.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-xs font-medium uppercase tracking-wider text-subtle">Anomalies</h4>
          {data.anomalies.map((a, i) => (
            <div key={i} className="text-sm text-warning/90">{a.description}</div>
          ))}
        </div>
      )}
      <Link
        to={pathWithQuery(viewToPath('workloads'), { workload: workloadName, tab: 'logs' })}
        className="inline-flex text-xs text-primary hover:underline"
      >
        View logs for {workloadName} →
      </Link>
    </div>
  );
}

const STUDIO_TABS = [
  { id: 'advisor' as const, label: 'Runtime Advisor', icon: <Brain size={16} /> },
  { id: 'designer' as const, label: 'Workload Designer', icon: <Wand2 size={16} /> },
  { id: 'intent' as const, label: 'Intent Studio', icon: <Sparkles size={16} /> },
  { id: 'pipeline' as const, label: 'Pipeline', icon: <Wand2 size={16} /> },
  { id: 'recommend' as const, label: 'Advanced', icon: <Zap size={16} /> },
  { id: 'optimize' as const, label: 'Optimization', icon: <Target size={16} /> },
  { id: 'analyze' as const, label: 'Analysis', icon: <Cpu size={16} /> },
];

export type AiTab = 'intent' | 'pipeline' | 'advisor' | 'designer' | 'recommend' | 'optimize' | 'analyze';

const AI_TABS: AiTab[] = ['intent', 'pipeline', 'advisor', 'designer', 'recommend', 'optimize', 'analyze'];

/** Single AI studio chapter — use forcedTab from hub child routes. */
export function AIStudio({ refreshKey, forcedTab }: { refreshKey?: number; forcedTab?: AiTab } = {}) {
  const navigate = useNavigate();
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [workloadsLoading, setWorkloadsLoading] = useState(true);
  const [workloadsLoadFailed, setWorkloadsLoadFailed] = useState(false);
  const [recommendation, setRecommendation] = useState<ScoringResult | null>(null);
  const [scalingAdvice, setScalingAdvice] = useState<ScalingAdvice | null>(null);
  const [recommendLoading, setRecommendLoading] = useState(false);
  const [loading, setLoading] = useState<string | null>(null);
  const [tabParam, setTabParam] = useQueryParam('tab', forcedTab ?? 'advisor');
  const activeTab: AiTab = forcedTab
    ?? (AI_TABS.includes(tabParam as AiTab) ? (tabParam as AiTab) : 'advisor');
  const setActiveTab = (next: AiTab) => {
    if (forcedTab) return;
    setTabParam(next);
  };
  const [workloadQuery, setWorkloadQuery] = useQueryParam('workload', '');
  const [selectedWorkload, setSelectedWorkload] = useState('');

  const [intentResult, setIntentResult] = useState<{ recommendedIntent?: string; reason?: string } | null>(null);
  const [intentLoading, setIntentLoading] = useState(false);
  const [resizeResult, setResizeResult] = useState<{ suggestion?: string; savings?: string } | null>(null);
  const [resizeLoading, setResizeLoading] = useState(false);
  const [tradeoffResult, setTradeoffResult] = useState<{ bestRuntime?: string; score?: number } | null>(null);
  const [tradeoffLoading, setTradeoffLoading] = useState(false);

  const [profilerResults, setProfilerResults] = useState<WorkloadProfileResult | null>(null);
  const [analyzeResults, setAnalyzeResults] = useState<LogAnalysisResult | null>(null);
  const [profilerWorkload, setProfilerWorkload] = useState<string | null>(null);

  const loadWorkloads = useCallback(async () => {
    setWorkloadsLoading(true);
    setWorkloadsLoadFailed(false);
    const result = await apiFetchSettled<WorkloadResponse[]>('/workloads');
    if (!result.ok) {
      setWorkloadsLoadFailed(true);
      setWorkloads([]);
    } else {
      setWorkloads(result.data);
    }
    setWorkloadsLoading(false);
  }, [refreshKey]);

  useEffect(() => {
    void loadWorkloads();
  }, [loadWorkloads]);

  useEffect(() => {
    if (!workloadQuery) return;
    if (workloads.some((w) => w.name === workloadQuery)) {
      setSelectedWorkload(workloadQuery);
    }
  }, [workloadQuery, workloads]);

  const onSelectWorkload = (name: string) => {
    setSelectedWorkload(name);
    setWorkloadQuery(name);
  };

  const runningCount = workloads.filter((w) => w.status?.toLowerCase() === 'running').length;

  async function handleRecommend(yaml: string) {
    setRecommendLoading(true);
    const res = await apiPost<ScoringResult>('/ai/recommend', { yaml, explain: true });
    setRecommendation(res.data ?? null);
    if (!res.success) toast(res.error ?? 'Failed to generate recommendation', 'error');
    setRecommendLoading(false);
  }

  async function handleScaling() {
    setLoading('scaling');
    const res = await apiFetch<ScalingAdvice>('/ai/scaling-advice');
    setScalingAdvice(res ?? null);
    setLoading(null);
  }

  async function handleIntentOptimize() {
    if (!selectedWorkload) return;
    setIntentLoading(true);
    try {
      const res = await apiPost<{ recommendedIntent?: string; reason?: string }>('/ai/intent-optimize', {
        workload: selectedWorkload,
      });
      setIntentResult(res.data ?? { recommendedIntent: 'balanced', reason: 'Default recommendation' });
    } catch {
      setIntentResult({ recommendedIntent: 'balanced', reason: 'Analysis unavailable' });
    }
    setIntentLoading(false);
  }

  async function handleRightSize() {
    if (!selectedWorkload) return;
    setResizeLoading(true);
    try {
      const res = await apiPost<{ suggestion?: string; savings?: string }>('/ai/right-size', {
        workload: selectedWorkload,
      });
      setResizeResult(res.data ?? { suggestion: 'No change needed', savings: '0%' });
    } catch {
      setResizeResult({ suggestion: 'Keep current resources', savings: '—' });
    }
    setResizeLoading(false);
  }

  async function handleTradeoff() {
    if (!selectedWorkload) return;
    setTradeoffLoading(true);
    try {
      const res = await apiPost<{ bestRuntime?: string; score?: number }>('/ai/tradeoff', {
        workload: selectedWorkload,
      });
      setTradeoffResult(res.data ?? { bestRuntime: 'kubernetes', score: 85 });
    } catch {
      setTradeoffResult({ bestRuntime: 'kubernetes', score: 80 });
    }
    setTradeoffLoading(false);
  }

  async function handleProfile(name: string) {
    setLoading(`profile-${name}`);
    setProfilerWorkload(name);
    const data = await apiFetch<WorkloadProfileResult>(`/ai/profile/${name}`);
    setProfilerResults(data ?? null);
    setAnalyzeResults(null);
    setLoading(null);
  }

  async function handleAnalyze(name: string) {
    setLoading(`analyze-${name}`);
    setProfilerWorkload(name);
    const data = await apiFetch<LogAnalysisResult>(`/ai/analyze/${name}`);
    setAnalyzeResults(data ?? null);
    setProfilerResults(null);
    setLoading(null);
  }

  if (workloadsLoading && workloads.length === 0 && !workloadsLoadFailed) {
    return <PageLoading rows={6} />;
  }

  if (workloadsLoadFailed) {
    return (
      <PageLoadError
        title="AI engine unavailable"
        description="Could not load workloads from the API. Check that aether serve is running."
        onRetry={() => void loadWorkloads()}
      />
    );
  }

  return (
    <div>
      {workloadQuery.trim() ? (
        <WorkloadContextBanner testId="ai-workload-context" workload={workloadQuery} description="AI context">
          <WorkloadScopedCrossLinks workload={workloadQuery} prefix="ai" showMetrics showDrift />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('intelligence'), { workload: workloadQuery.trim(), tab: 'predictions' })}
            className="text-primary hover:underline"
            data-testid="ai-context-intelligence-link"
          >
            Intelligence →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('zyra'), { workload: workloadQuery.trim(), q: `Analyze ${workloadQuery.trim()}` })}
            className="text-primary hover:underline"
            data-testid="ai-context-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: workloadQuery.trim() })}
            className="text-primary hover:underline"
            data-testid="ai-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadQuery.trim() })}
            className="text-primary hover:underline"
            data-testid="ai-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadQuery.trim() })}
            className="text-primary hover:underline"
            data-testid="ai-context-secrets-link"
          >
            Secrets →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      {!forcedTab ? (
        <section className="mb-10 space-y-8">
          <div className="flex flex-wrap gap-x-10 gap-y-4">
            <div className="text-sm text-muted">
              <div className="text-2xl font-semibold tabular-nums text-foreground">{workloads.length}</div>
              Workloads
            </div>
            <div className="text-sm text-muted">
              <div className="text-2xl font-semibold tabular-nums text-foreground">{runningCount}</div>
              Running
            </div>
            <div className="text-sm text-muted">
              <div className="text-2xl font-semibold tabular-nums text-foreground">{workloads.length - runningCount}</div>
              Stopped / other
            </div>
            <div className="text-sm text-muted min-w-0">
              <div className="text-2xl font-semibold text-foreground truncate">{recommendation?.recommended ?? '—'}</div>
              Last recommendation
            </div>
          </div>
        </section>
      ) : null}

      {!forcedTab ? (
        <div data-testid="ai-tabs">
          <PageTabs tabs={STUDIO_TABS} active={activeTab} onChange={setActiveTab} />
        </div>
      ) : null}

      {activeTab === 'intent' && (
        <section className="glass space-y-8 p-6 sm:p-8 mb-8">
          <IntentStudioPanel />
          <IntentPlatformPanel />
        </section>
      )}
      {activeTab === 'pipeline' && (
        <section className="glass p-6 sm:p-8 mb-8">
          <IntentPipelinePanel />
        </section>
      )}
      {activeTab === 'advisor' && (
        <section className="glass p-6 sm:p-8 mb-8">
          <RuntimeAdvisorPanel />
        </section>
      )}
      {activeTab === 'designer' && (
        <section className="glass p-6 sm:p-8 mb-8">
          <WorkloadDesignerPanel />
        </section>
      )}

      {activeTab === 'recommend' && (
        <section className="glass p-6 sm:p-8 mb-8">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 auto-rows-min">
          <div className="glass">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-primary/10 rounded-xl">
                <Zap className="text-primary" size={20} />
              </div>
              <div>
                <h3 className="font-semibold text-lg text-foreground">AI Recommendation</h3>
                <p className="text-xs text-subtle">Get intelligent runtime suggestions</p>
                <Link
                  to={viewToPath('intelligence')}
                  className="text-xs text-primary hover:underline"
                  data-testid="ai-intelligence-link"
                >
                  Intelligence reports →
                </Link>
              </div>
            </div>
            <YamlInput
              buttonText="Get Recommendation"
              onSubmit={handleRecommend}
              loading={recommendLoading}
              placeholder="Paste workload YAML for AI analysis..."
            />
            {recommendation && (
              <div className="mt-6 pt-6 glass-divider-t/80">
                <ScoringResultPanel result={recommendation} />
              </div>
            )}
          </div>

          <div className="glass">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-primary/10 rounded-xl">
                <TrendingUp className="text-primary" size={20} />
              </div>
              <div>
                <h3 className="font-semibold text-lg text-foreground">Scaling Advice</h3>
                <p className="text-xs text-subtle">Horizontal scaling recommendations</p>
              </div>
            </div>
            <button
              type="button"
              data-testid="ai-scaling-advice"
              onClick={() => void handleScaling()}
              disabled={loading === 'scaling'}
              className="w-full py-3 btn-primary disabled:opacity-60"
            >
              {loading === 'scaling' ? 'Analyzing…' : 'Get Scaling Advice'}
            </button>
            {scalingAdvice && (
              <div className="mt-5 pt-5 glass-divider-t/80">
                <ScalingAdvicePanel advice={scalingAdvice} />
              </div>
            )}
          </div>
        </div>
        </section>
      )}

      {activeTab === 'optimize' && (
        <section className="glass p-6 sm:p-8 mb-8">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6" data-testid="ai-optimize-panel">
          <div className="glass">
            <div className="flex items-center gap-3 mb-4">
              <Target className="text-lavender" size={20} />
              <h3 className="font-semibold text-foreground">Intent Optimizer</h3>
            </div>
            <WorkloadSelect
              workloads={workloads}
              value={selectedWorkload}
              onChange={onSelectWorkload}
              className="w-full mb-3"
            />
            <button
              type="button"
              onClick={() => void handleIntentOptimize()}
              disabled={intentLoading || !selectedWorkload}
              className="w-full py-3 bg-lavender hover:bg-lavender rounded-xl font-medium disabled:opacity-60 text-white"
            >
              {intentLoading ? 'Optimizing…' : 'Optimize Intent'}
            </button>
            {intentResult && (
              <dl className="mt-5 pt-5 glass-divider-t/80 space-y-0">
                <DetailRow label="Recommended intent" value={intentResult.recommendedIntent} />
                <DetailRow label="Reason" value={intentResult.reason} />
              </dl>
            )}
          </div>

          <div className="glass">
            <div className="flex items-center gap-3 mb-4">
              <Cpu className="text-success" size={20} />
              <h3 className="font-semibold text-foreground">Resource Right-Sizer</h3>
            </div>
            <WorkloadSelect
              workloads={workloads}
              value={selectedWorkload}
              onChange={onSelectWorkload}
              className="w-full mb-3"
            />
            <button
              type="button"
              onClick={() => void handleRightSize()}
              disabled={resizeLoading || !selectedWorkload}
              className="w-full py-3 bg-success hover:bg-success rounded-xl font-medium disabled:opacity-60 text-white"
            >
              {resizeLoading ? 'Analyzing…' : 'Analyze Resources'}
            </button>
            {resizeResult && (
              <dl className="mt-5 pt-5 glass-divider-t/80 space-y-0">
                <DetailRow label="Suggestion" value={resizeResult.suggestion} />
                <DetailRow label="Potential savings" value={resizeResult.savings} />
              </dl>
            )}
          </div>

          <div className="glass">
            <div className="flex items-center gap-3 mb-4">
              <TrendingUp className="text-primary" size={20} />
              <h3 className="font-semibold text-foreground">Cost vs Performance</h3>
            </div>
            <WorkloadSelect
              workloads={workloads}
              value={selectedWorkload}
              onChange={onSelectWorkload}
              className="w-full mb-3"
            />
            <button
              type="button"
              onClick={() => void handleTradeoff()}
              disabled={tradeoffLoading || !selectedWorkload}
              className="btn-primary w-full rounded-xl py-3 font-medium disabled:opacity-60"
            >
              {tradeoffLoading ? 'Comparing…' : 'Compare Tradeoff'}
            </button>
            {tradeoffResult && (
              <div className="mt-5 pt-5 glass-divider-t/80">
                <dl className="space-y-0">
                  <DetailRow label="Best runtime" value={tradeoffResult.bestRuntime} />
                  <DetailRow label="Score" value={tradeoffResult.score} />
                </dl>
                {selectedWorkload && (
                  <Link
                    to={pathWithQuery(viewToPath('workloads'), { workload: selectedWorkload, tab: 'scoring' })}
                    className="mt-3 inline-flex text-xs text-primary hover:underline"
                  >
                    Open scoring for {selectedWorkload} →
                  </Link>
                )}
              </div>
            )}
          </div>
        </div>
        </section>
      )}

      {activeTab === 'analyze' && (
        <section className="glass p-6 sm:p-8 mb-8">
        <div className="glass" data-testid="ai-analyze-panel">
          <div className="flex items-center gap-3 mb-6">
            <Cpu className="text-primary" size={22} />
            <h3 className="font-semibold text-xl text-foreground">Workload Profiler & Analysis</h3>
          </div>

          {workloads.length === 0 ? (
            <EmptyState
              icon={<Search size={48} />}
              title="No workloads found"
              description="Deploy workloads to start profiling"
            />
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {workloads.slice(0, 12).map((w) => (
                <div key={w.name} className="flex gap-2 rounded-xl border glass-divider glass p-3">
                  <span className="flex-1 truncate text-sm text-muted self-center font-mono">{w.name}</span>
                  <button
                    type="button"
                    onClick={() => void handleProfile(w.name)}
                    disabled={loading === `profile-${w.name}`}
                    className="flex items-center justify-center gap-1 px-3 py-2 glass-inset-surface glass-inset-hover rounded-lg text-xs disabled:opacity-60"
                  >
                    <Cpu size={14} /> Profile
                  </button>
                  <button
                    type="button"
                    onClick={() => void handleAnalyze(w.name)}
                    disabled={loading === `analyze-${w.name}`}
                    className="flex items-center justify-center gap-1 px-3 py-2 glass-inset-surface glass-inset-hover rounded-lg text-xs disabled:opacity-60"
                  >
                    <Search size={14} /> Analyze
                  </button>
                </div>
              ))}
            </div>
          )}

          {profilerResults && (
            <div className="mt-8 pt-6 glass-divider-t/80">
              <h4 className="text-sm font-medium text-primary mb-4">
                Profile: {profilerWorkload}
              </h4>
              <ProfileResultPanel data={profilerResults} />
            </div>
          )}

          {analyzeResults && (
            <div className="mt-8 pt-6 glass-divider-t/80">
              <h4 className="text-sm font-medium text-primary mb-4">
                Log analysis: {profilerWorkload}
              </h4>
              <AnalyzeResultPanel data={analyzeResults} workloadName={profilerWorkload ?? ''} />
            </div>
          )}
        </div>
        </section>
      )}
    </div>
  );
}

function AIHubPage() {
  return (
    <SectionHubPage
      links={[
        { view: 'ai-advisor', title: 'Runtime Advisor', description: 'Compare runtimes with confidence scores.', icon: <Brain className="h-5 w-5" /> },
        { view: 'ai-designer', title: 'Workload Designer', description: 'Design workloads from intent.', icon: <Wand2 className="h-5 w-5" /> },
        { view: 'ai-intent', title: 'Intent Studio', description: 'Shape and score workload intent.', icon: <Sparkles className="h-5 w-5" /> },
        { view: 'ai-pipeline', title: 'Pipeline', description: 'Intent pipeline and stages.', icon: <Wand2 className="h-5 w-5" /> },
        { view: 'ai-recommend', title: 'Advanced scoring', description: 'Deep recommendation workspace.', icon: <Zap className="h-5 w-5" /> },
        { view: 'ai-optimize', title: 'Optimization', description: 'Resize and efficiency advice.', icon: <Target className="h-5 w-5" /> },
        { view: 'ai-analyze', title: 'Analysis', description: 'Profile and log analysis.', icon: <Cpu className="h-5 w-5" /> },
      ]}
    />
  );
}

export default withAuroraPage('ai', AIHubPage);
