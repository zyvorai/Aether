import { useState, useEffect } from 'react';
import { TrendingUp, Cpu, Search, Inbox, ArrowRightLeft } from 'lucide-react';
import { apiFetch, apiPost } from '../../utils/api';
import YamlInput from '../YamlInput';
import CodeBlock from '../CodeBlock';
import EmptyState from '../EmptyState';
import Badge, { RuntimeBadge } from '../Badge';
import BarChart from '../BarChart';
import Modal from '../Modal';
import type { WorkloadResponse, ScoringResult, ScalingAdvice, MigrationAdvice } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function AIPage() {
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [recommendation, setRecommendation] = useState<ScoringResult | null>(null);
  const [scalingAdvice, setScalingAdvice] = useState<ScalingAdvice | null>(null);
  const [profilerResults, setProfilerResults] = useState<Record<string, unknown> | null>(null);
  const [analyzeResults, setAnalyzeResults] = useState<Record<string, unknown> | null>(null);
  const [loading, setLoading] = useState<string | null>(null);
  const [recommendLoading, setRecommendLoading] = useState(false);

  // Migration Advice state
  const [migrationWorkload, setMigrationWorkload] = useState('');
  const [migrationTarget, setMigrationTarget] = useState('');
  const [migrationAdvice, setMigrationAdvice] = useState<MigrationAdvice | null>(null);
  const [migrationLoading, setMigrationLoading] = useState(false);
  const [migrationModalOpen, setMigrationModalOpen] = useState(false);

  const runtimes = ['podman', 'docker', 'kubernetes', 'kubevirt', 'metal3'];

  useEffect(() => {
    async function load() {
      const data = await apiFetch<WorkloadResponse[]>('/workloads');
      setWorkloads(data ?? []);
    }
    load();
  }, []);

  async function handleRecommend(yaml: string) {
    setRecommendLoading(true);
    const res = await apiPost<ScoringResult>('/ai/recommend', { yaml });
    setRecommendation(res.data ?? null);
    if (!res.success) {
      toast(res.error ?? 'Failed to generate recommendation', 'error');
    }
    setRecommendLoading(false);
  }

  async function handleScaling() {
    setLoading('scaling');
    const res = await apiFetch<ScalingAdvice>('/ai/scaling-advice');
    setScalingAdvice(res ?? null);
    setLoading(null);
  }

  async function handleProfile(name: string) {
    setLoading(`profile-${name}`);
    const data = await apiFetch<Record<string, unknown>>(`/ai/profile/${name}`);
    setProfilerResults(data);
    if (data) {
      toast(`Profile for "${name}" loaded`, 'success');
    } else {
      toast(`Failed to load profile for "${name}"`, 'error');
    }
    setLoading(null);
  }

  async function handleAnalyze(name: string) {
    setLoading(`analyze-${name}`);
    const data = await apiFetch<Record<string, unknown>>(`/ai/analyze/${name}`);
    setAnalyzeResults(data);
    if (data) {
      toast(`Analysis for "${name}" loaded`, 'success');
    } else {
      toast(`Failed to analyze "${name}"`, 'error');
    }
    setLoading(null);
  }

  async function handleMigrationAdvice() {
    if (!migrationWorkload || !migrationTarget) return;
    setMigrationLoading(true);
    const data = await apiFetch<MigrationAdvice>(`/ai/migration-advice/${migrationWorkload}/${migrationTarget}`);
    if (data) {
      setMigrationAdvice(data);
      setMigrationModalOpen(true);
      toast(`Migration advice for "${migrationWorkload}" loaded`, 'success');
    } else {
      toast(`Failed to get migration advice for "${migrationWorkload}"`, 'error');
    }
    setMigrationLoading(false);
  }

  return (
    <div>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        {/* AI Recommendation */}
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4">AI Recommendation</h2>
          <YamlInput
            buttonText="Get Recommendation"
            onSubmit={handleRecommend}
            loading={recommendLoading}
            placeholder="Paste workload YAML for AI analysis..."
          />

          {recommendation && (
            <div className="mt-4 space-y-3">
              <div className="flex items-center gap-2">
                <span className="text-sm text-zinc-400">Recommended:</span>
                <RuntimeBadge runtime={recommendation.recommended} />
                <Badge
                  text={`${(recommendation.confidence * 100).toFixed(0)}% confidence`}
                  variant="green"
                />
              </div>
              <div className="text-xs text-zinc-500">Class: {recommendation.workload_class}</div>
              <div className="space-y-2">
                {recommendation.scores.map((s) => (
                  <div key={s.runtime} className="bg-zinc-950/50 rounded-lg p-3">
                    <div className="flex items-center justify-between mb-2">
                      <RuntimeBadge runtime={s.runtime} />
                      <span className="text-sm font-medium text-zinc-300">{s.total_score.toFixed(1)}</span>
                    </div>
                    <div className="grid grid-cols-2 gap-2">
                      <BarChart label="Cost" percent={s.cost_score * 100} />
                      <BarChart label="Performance" percent={s.performance_score * 100} />
                      <BarChart label="Reliability" percent={s.reliability_score * 100} />
                      <BarChart label="Availability" percent={s.availability_score * 100} />
                    </div>
                    {s.reasons.length > 0 && (
                      <ul className="mt-2 space-y-0.5">
                        {s.reasons.map((r, i) => (
                          <li key={i} className="text-xs text-zinc-500">+ {r}</li>
                        ))}
                      </ul>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Scaling Advice */}
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <h2 className="text-lg font-semibold text-zinc-100 mb-4 flex items-center gap-2">
            <TrendingUp size={20} className="text-blue-400" />
            Scaling Advice
          </h2>
          <button
            onClick={handleScaling}
            disabled={loading === 'scaling'}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
          >
            {loading === 'scaling' ? 'Loading...' : 'Get Scaling Advice'}
          </button>

          {scalingAdvice && (
            <div className="mt-4 space-y-3">
              <div className="flex items-center gap-2">
                <Badge
                  text={scalingAdvice.action}
                  variant={scalingAdvice.action === 'scale_up' ? 'yellow' : scalingAdvice.action === 'scale_down' ? 'blue' : 'green'}
                />
                <Badge
                  text={`${(scalingAdvice.confidence * 100).toFixed(0)}% confidence`}
                  variant="muted"
                />
              </div>
              <div className="bg-zinc-950/50 rounded-lg p-3 space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-zinc-400">Current replicas</span>
                  <span className="text-zinc-200">{scalingAdvice.current_replicas}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-zinc-400">Recommended replicas</span>
                  <span className="text-amber-400 font-medium">{scalingAdvice.recommended_replicas}</span>
                </div>
                <div className="text-sm text-zinc-400">{scalingAdvice.reason}</div>
              </div>
              {scalingAdvice.forecast && (
                <CodeBlock title="Forecast">{JSON.stringify(scalingAdvice.forecast, null, 2)}</CodeBlock>
              )}
              {scalingAdvice.cost_impact && (
                <CodeBlock title="Cost Impact">{JSON.stringify(scalingAdvice.cost_impact, null, 2)}</CodeBlock>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Migration Advice */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 mb-6">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4 flex items-center gap-2">
          <ArrowRightLeft size={20} className="text-purple-400" />
          Migration Advice
        </h2>
        <div className="flex flex-wrap items-end gap-3">
          <div>
            <label className="block text-xs text-zinc-400 mb-1">Workload</label>
            <select
              value={migrationWorkload}
              onChange={(e) => setMigrationWorkload(e.target.value)}
              className="px-3 py-2 bg-zinc-950 border border-zinc-800 rounded-lg text-sm text-zinc-200 focus:outline-none focus:border-amber-500"
            >
              <option value="">Select workload</option>
              {workloads.map((w) => (
                <option key={w.name} value={w.name}>{w.name}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-zinc-400 mb-1">Target Runtime</label>
            <select
              value={migrationTarget}
              onChange={(e) => setMigrationTarget(e.target.value)}
              className="px-3 py-2 bg-zinc-950 border border-zinc-800 rounded-lg text-sm text-zinc-200 focus:outline-none focus:border-amber-500"
            >
              <option value="">Select runtime</option>
              {runtimes.map((rt) => (
                <option key={rt} value={rt}>{rt}</option>
              ))}
            </select>
          </div>
          <button
            onClick={handleMigrationAdvice}
            disabled={migrationLoading || !migrationWorkload || !migrationTarget}
            className="px-4 py-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
          >
            {migrationLoading ? 'Loading...' : 'Get Advice'}
          </button>
        </div>
      </div>

      {/* Migration Advice Modal */}
      <Modal
        isOpen={migrationModalOpen}
        onClose={() => { setMigrationModalOpen(false); setMigrationAdvice(null); }}
        title={`Migration Advice: ${migrationAdvice?.workload_name ?? ''}`}
      >
        {migrationAdvice && (
          <div className="space-y-4">
            <div className="flex items-center gap-2 flex-wrap">
              <Badge text={migrationAdvice.recommended_strategy} variant="blue" />
              <Badge text={`Risk: ${migrationAdvice.risk_level}`} variant={migrationAdvice.risk_level === 'low' ? 'green' : migrationAdvice.risk_level === 'high' ? 'red' : 'yellow'} />
              <span className="text-xs text-zinc-400">Downtime: ~{migrationAdvice.estimated_downtime_secs}s</span>
            </div>
            <div className="text-sm text-zinc-300">
              <span className="text-zinc-500">From</span> {migrationAdvice.source_runtime} <span className="text-zinc-500">to</span> {migrationAdvice.target_runtime}
            </div>
            {migrationAdvice.reasons.length > 0 && (
              <div>
                <h3 className="text-sm font-medium text-zinc-300 mb-1">Reasons</h3>
                <ul className="space-y-0.5">
                  {migrationAdvice.reasons.map((r, i) => (
                    <li key={i} className="text-xs text-zinc-400">+ {r}</li>
                  ))}
                </ul>
              </div>
            )}
            {migrationAdvice.warnings.length > 0 && (
              <div>
                <h3 className="text-sm font-medium text-zinc-300 mb-1">Warnings</h3>
                <ul className="space-y-0.5">
                  {migrationAdvice.warnings.map((w, i) => (
                    <li key={i} className="text-xs text-amber-400">! {w}</li>
                  ))}
                </ul>
              </div>
            )}
            <CodeBlock title="Full Advice">{JSON.stringify(migrationAdvice, null, 2)}</CodeBlock>
          </div>
        )}
      </Modal>

      {/* Profiler */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
        <h2 className="text-lg font-semibold text-zinc-100 mb-4 flex items-center gap-2">
          <Cpu size={20} className="text-cyan-400" />
          Workload Profiler
        </h2>
        {workloads.length === 0 ? (
          <EmptyState icon={<Inbox size={48} />} title="No workloads" description="Deploy a workload to profile it" />
        ) : (
          <div className="flex flex-wrap gap-2 mb-4">
            {workloads.map((w) => (
              <div key={w.name} className="flex gap-1">
                <button
                  onClick={() => handleProfile(w.name)}
                  disabled={loading === `profile-${w.name}`}
                  className="flex items-center gap-1.5 px-3 py-2 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-lg text-sm text-zinc-200 transition-colors"
                >
                  <Cpu size={14} />
                  Profile {w.name}
                </button>
                <button
                  onClick={() => handleAnalyze(w.name)}
                  disabled={loading === `analyze-${w.name}`}
                  className="flex items-center gap-1.5 px-3 py-2 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-lg text-sm text-zinc-200 transition-colors"
                >
                  <Search size={14} />
                  Analyze
                </button>
              </div>
            ))}
          </div>
        )}

        {profilerResults && (
          <div className="mt-4">
            <CodeBlock title="Profile Results">{JSON.stringify(profilerResults, null, 2)}</CodeBlock>
          </div>
        )}

        {analyzeResults && (
          <div className="mt-4">
            <CodeBlock title="Analysis Results">{JSON.stringify(analyzeResults, null, 2)}</CodeBlock>
          </div>
        )}
      </div>
    </div>
  );
}
