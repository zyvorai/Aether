// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { ArrowRight, Loader2, Play, Sparkles, Wand2 } from 'lucide-react';
import { apiPost } from '../utils/api';
import { formatPercent } from '../utils/formatters';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import type { IntentDeployReport, IntentPipelineReport, NlIntentReport } from '../types/api';
import GlassSection from './GlassSection';

const GOALS = [
  { id: 'performance', label: 'Low latency' },
  { id: 'cost', label: 'Cost optimized' },
  { id: 'availability', label: 'High availability' },
  { id: 'throughput', label: 'High throughput' },
] as const;

export default function IntentPipelinePanel() {
  const [selected, setSelected] = useState<Set<string>>(new Set(['performance', 'availability']));
  const [workloadName, setWorkloadName] = useState('my-app');
  const [report, setReport] = useState<IntentPipelineReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [nlText, setNlText] = useState('Cost-optimized API with 99.9% availability under $500/month');
  const [nlReport, setNlReport] = useState<NlIntentReport | null>(null);
  const [deploying, setDeploying] = useState(false);

  const runPipeline = useCallback(async () => {
    setLoading(true);
    const res = await apiPost<IntentPipelineReport>('/intelligence/intent-pipeline', {
      goals: GOALS.filter((g) => selected.has(g.id)).map((g) => g.id),
      workload_name: workloadName.trim() || 'my-app',
    });
    setReport(res.success ? res.data ?? null : null);
    setLoading(false);
  }, [selected, workloadName]);

  useEffect(() => {
    void runPipeline();
  }, [runPipeline]);

  function toggle(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function parseNl() {
    setLoading(true);
    const res = await apiPost<NlIntentReport>('/intelligence/intent/nl-parse', {
      text: nlText,
      workload_name: workloadName.trim() || 'my-app',
    });
    setNlReport(res.success ? res.data ?? null : null);
    setLoading(false);
  }

  async function deployPipeline(dryRun: boolean) {
    setDeploying(true);
    const res = await apiPost<IntentDeployReport>('/intelligence/intent-pipeline/deploy', {
      goals: GOALS.filter((g) => selected.has(g.id)).map((g) => g.id),
      workload_name: workloadName.trim() || 'my-app',
      dry_run: dryRun,
    });
    setDeploying(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Deploy dry-run: ${res.data?.steps.length ?? 0} steps`
              : res.data?.blocked
                ? `Deploy blocked: ${res.data?.block_reason ?? 'policy'}`
                : `Spec ready at ${res.data?.spec_path ?? 'path'}`,
            type: res.data?.blocked ? 'error' : 'success',
          },
        }),
      );
    }
  }

  return (
        <GlassSection
      accent="purple"
      testId="intent-pipeline-panel"
      title="Intent → Infrastructure"
      subtitle="Describe outcomes — Aether generates spec, runtime, placement, and deploy steps."
      icon={<Wand2 className="h-5 w-5 text-violet-400" />}
      actions={<div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void deployPipeline(true)}
            disabled={deploying}
            className="rounded-xl border border-violet-500/30 bg-violet-500/10 px-3 py-2 text-xs text-violet-200 disabled:opacity-60"
            data-testid="intent-pipeline-deploy-dry-run"
          >
            Dry-run deploy
          </button>
          <button
            type="button"
            onClick={() => void runPipeline()}
            disabled={loading}
            className="inline-flex items-center gap-2 rounded-xl bg-aether px-4 py-2 text-sm font-medium text-white hover:bg-aether/90 disabled:opacity-60"
          >
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
            Run pipeline
          </button>
        </div>}
    ><label className="mb-6 block text-sm" data-testid="intent-nl-input">
        <span className="mb-1 block text-xs text-slate-500">Natural language intent</span>
        <textarea
          value={nlText}
          onChange={(e) => setNlText(e.target.value)}
          rows={2}
          className="glass-input"
        />
        <button
          type="button"
          onClick={() => void parseNl()}
          disabled={loading}
          className="mt-2 rounded-lg border border-slate-700 px-3 py-1 text-xs text-slate-300 hover:border-aether/40"
        >
          Parse NL → intent block
        </button>
        {nlReport ? (
          <pre className="mt-2 max-h-32 overflow-auto rounded-lg bg-black/40 p-2 text-[11px] text-slate-400">
            {nlReport.intent_yaml}
          </pre>
        ) : null}
      </label>

      <div className="mb-6 grid gap-4 md:grid-cols-2">
        <label className="block text-sm">
          <span className="mb-1 block text-xs text-slate-500">Workload name</span>
          <input
            value={workloadName}
            onChange={(e) => setWorkloadName(e.target.value)}
            className="glass-input"
          />
        </label>
        <div>
          <span className="mb-2 block text-xs text-slate-500">Outcomes</span>
          <div className="flex flex-wrap gap-2">
            {GOALS.map((g) => (
              <button
                key={g.id}
                type="button"
                onClick={() => toggle(g.id)}
                className={`rounded-full border px-3 py-1 text-xs ${
                  selected.has(g.id)
                    ? 'border-violet-500/40 bg-violet-500/10 text-violet-200'
                    : 'border-slate-700 text-slate-400'
                }`}
              >
                {g.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {report ? (
        <div className="space-y-6">
          <div className="grid gap-3 sm:grid-cols-3">
            <div className="glass-metric-card">
              <Sparkles className="mb-2 h-4 w-4 text-violet-400" />
              <div className="text-lg font-semibold text-white">{report.recommended_runtime}</div>
              <div className="text-xs text-slate-500">Recommended runtime</div>
            </div>
            <div className="glass-metric-card">
              <div className="text-lg font-semibold text-white">{formatPercent(report.confidence * 100, 0)}</div>
              <div className="text-xs text-slate-500">Confidence</div>
            </div>
            <div className="glass-metric-card">
              <div className="text-lg font-semibold text-white">{report.placement[0]?.cluster ?? 'local'}</div>
              <div className="text-xs text-slate-500">Top cluster</div>
            </div>
          </div>

          <div>
            <p className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500">Pipeline steps</p>
            <ol className="space-y-3">
              {report.steps.map((step) => (
                <li key={step.phase} className="glass-panel-card px-4 py-3">
                  <div className="flex items-center gap-2 text-sm font-medium text-white">
                    <span className="rounded bg-slate-800 px-2 py-0.5 text-[10px] uppercase text-slate-400">{step.phase}</span>
                    {step.title}
                  </div>
                  <p className="mt-1 text-sm text-slate-400">{step.detail}</p>
                  <p className="mt-1 text-xs text-aether">{step.action}</p>
                </li>
              ))}
            </ol>
          </div>

          <div className="flex flex-wrap gap-3">
            <Link
              to={pathWithQuery(viewToPath('editor'), { workload: report.workload_name })}
              className="inline-flex items-center gap-1 text-sm text-aether hover:underline"
            >
              Open in editor <ArrowRight className="h-3 w-3" />
            </Link>
            <Link to={viewToPath('fabric')} className="inline-flex items-center gap-1 text-sm text-aether hover:underline">
              Runtime fabric <ArrowRight className="h-3 w-3" />
            </Link>
          </div>

          <pre className="max-h-48 overflow-auto rounded-xl border border-slate-800 bg-black/40 p-3 text-xs text-slate-300">
            {report.spec_yaml}
          </pre>
        </div>
      ) : null}
    </GlassSection>
  );
}
