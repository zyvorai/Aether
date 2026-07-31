// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState } from 'react';
import { Target } from 'lucide-react';
import { apiPost } from '../utils/api';
import YamlInput from './YamlInput';
import Badge, { RuntimeBadge } from './Badge';
import BarChart from './BarChart';
import type { ScoringResult, RuntimeScore } from '../types/api';
import { formatPercent } from '../utils/formatters';
import GlassSection from './GlassSection';

function confidenceVariant(confidence: number): 'green' | 'yellow' | 'muted' {
  if (confidence >= 0.8) return 'green';
  if (confidence >= 0.6) return 'yellow';
  return 'muted';
}

function RuntimeScoreBlock({ score, recommended }: { score: RuntimeScore; recommended: string }) {
  const isRecommended = score.runtime === recommended;
  return (
    <div className={`rounded-xl border p-3 ${isRecommended ? 'border-aether/40 bg-aether/5' : 'glass-divider glass-panel-card'}`}>
      <div className="flex items-center justify-between gap-2 mb-3">
        <RuntimeBadge runtime={score.runtime} />
        {isRecommended ? <Badge text="Recommended" variant="accent" /> : null}
      </div>
      <BarChart label="Overall score" percent={score.total_score * 100} />
      {(score.reasons ?? []).length > 0 ? (
        <ul className="mt-3 space-y-1 text-xs text-slate-400">
          {score.reasons.map((r, i) => (
            <li key={i} className="flex gap-2">
              <span className="text-emerald-400 shrink-0">✓</span>
              <span>{r}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

export default function RuntimeAdvisorPanel() {
  const [result, setResult] = useState<ScoringResult | null>(null);
  const [loading, setLoading] = useState(false);

  async function analyze(yaml: string) {
    setLoading(true);
    const res = await apiPost<ScoringResult>('/ai/recommend', { yaml, explain: true });
    setResult(res.data ?? null);
    setLoading(false);
  }

  const sorted = result ? [...result.scores].sort((a, b) => b.total_score - a.total_score) : [];

  return (
    <section className="space-y-6" data-testid="runtime-advisor-panel">
      <GlassSection
        accent="purple"
        title="AI Runtime Advisor"
        subtitle="Recommended runtime with reasons, warnings, and confidence."
        icon={<Target className="h-5 w-5 text-aether" />}
      >
        <YamlInput
          buttonText="Analyze placement"
          onSubmit={(yaml) => void analyze(yaml)}
          loading={loading}
          placeholder="Paste workload YAML for runtime scoring…"
        />
        {result ? (
          <div className="mt-6 space-y-4 glass-divider-t/60 pt-6">
            <div className="flex flex-wrap items-center gap-2">
              <RuntimeBadge runtime={result.recommended} />
              <Badge
                text={`${formatPercent(result.confidence, 0)} confidence`}
                variant={confidenceVariant(result.confidence)}
              />
              <Badge text={result.workload_class} variant="muted" />
            </div>
            <div className="grid gap-3 lg:grid-cols-2">
              {sorted.map((s) => (
                <RuntimeScoreBlock key={s.runtime} score={s} recommended={result.recommended} />
              ))}
            </div>
          </div>
        ) : null}
      </GlassSection>
    </section>
  );
}
