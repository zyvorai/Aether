// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useMemo, useState } from 'react';
import { Sparkles } from 'lucide-react';
import { apiPost } from '../utils/api';
import GlassSection from './GlassSection';

const GOALS = [
  { id: 'performance', label: 'Performance', weight: 'high-latency' },
  { id: 'cost', label: 'Cost', weight: 'cost-optimized' },
  { id: 'availability', label: 'Availability', weight: 'high-availability' },
  { id: 'compliance', label: 'Compliance', weight: 'compliance' },
  { id: 'security', label: 'Security', weight: 'security-hardened' },
  { id: 'gpu', label: 'GPU', weight: 'gpu' },
] as const;

function buildIntentYaml(selected: Set<string>, workloadName: string): string {
  const intents = GOALS.filter((g) => selected.has(g.id)).map((g) => g.weight);
  const intentBlock =
    intents.length > 0
      ? intents.map((i) => `  - ${i}`).join('\n')
      : '  - balanced';

  return `apiVersion: aether/v1
kind: Workload
metadata:
  name: ${workloadName || 'my-app'}
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
requirements:
  cpu: 500m
  memory: 512Mi
  storage: 1Gi
intent:
${intentBlock}
runtime:
  preferred: auto
  allow:
    - kube
    - podman
`;
}

export default function IntentStudioPanel() {
  const [selected, setSelected] = useState<Set<string>>(new Set(['performance', 'availability']));
  const [workloadName, setWorkloadName] = useState('my-app');
  const [generatedYaml, setGeneratedYaml] = useState('');
  const [generating, setGenerating] = useState(false);
  const [pipelineSummary, setPipelineSummary] = useState<string | null>(null);

  const yaml = useMemo(
    () => buildIntentYaml(selected, workloadName.trim() || 'my-app'),
    [selected, workloadName],
  );

  function toggle(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  return (
    <GlassSection
      accent="purple"
      testId="intent-studio-panel"
      title="Intent Studio"
      subtitle="What matters? Aether builds the intent block — no YAML editing required."
      icon={<Sparkles className="h-5 w-5 text-violet-400" />}
    >
      <label className="mb-4 block">
          <span className="mb-2 block text-xs font-medium uppercase tracking-wider text-ink-3">Workload name</span>
          <input
            value={workloadName}
            onChange={(e) => setWorkloadName(e.target.value)}
            className="glass-input max-w-md"
            placeholder="my-app"
          />
        </label>

        <div className="mb-6">
          <span className="mb-3 block text-xs font-medium uppercase tracking-wider text-ink-3">What matters?</span>
          <div className="flex flex-wrap gap-2">
            {GOALS.map((goal) => {
              const active = selected.has(goal.id);
              return (
                <button
                  key={goal.id}
                  type="button"
                  onClick={() => toggle(goal.id)}
                  className={`rounded-full border px-4 py-2 text-sm transition ${
                    active
                      ? 'border-violet-500/50 bg-violet-500/15 text-violet-100'
                      : 'glass-divider glass-panel-card text-ink-2 hover:border-brand/30'
                  }`}
                  data-testid={`intent-goal-${goal.id}`}
                >
                  {goal.label}
                </button>
              );
            })}
          </div>
        </div>

        <button
          type="button"
          onClick={() => {
            void (async () => {
              setGenerating(true);
              setPipelineSummary(null);
              const goals = GOALS.filter((g) => selected.has(g.id)).map((g) => g.weight);
              const res = await apiPost<{ steps?: Array<{ label: string; status: string }>; workload_name?: string }>(
                '/intelligence/intent-pipeline',
                { goals, workload_name: workloadName.trim() || 'my-app' },
              );
              setGeneratedYaml(yaml);
              if (res.success && res.data?.steps?.length) {
                setPipelineSummary(
                  res.data.steps.map((step) => `${step.label}: ${step.status}`).join(' · '),
                );
              }
              setGenerating(false);
            })();
          }}
          disabled={generating}
          className="rounded-xl bg-violet-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-violet-500 disabled:opacity-60"
          data-testid="intent-generate-button"
        >
          {generating ? 'Generating…' : 'Generate Intent'}
        </button>

        {pipelineSummary ? (
          <p className="mt-3 text-xs text-violet-200/90" data-testid="intent-pipeline-summary">
            Pipeline: {pipelineSummary}
          </p>
        ) : null}

        {generatedYaml ? (
          <pre className="glass-code-block-body mt-6 text-xs text-ink-2" data-testid="intent-generated-yaml">
            {generatedYaml}
          </pre>
        ) : null}
    </GlassSection>
  );
}
