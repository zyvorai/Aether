// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useMemo, useState } from 'react';
import { Sparkles } from 'lucide-react';

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
    <section className="space-y-6" data-testid="intent-studio-panel">
      <div className="surface-panel rounded-[28px] p-6 sm:p-8">
        <div className="mb-6 flex items-center gap-3">
          <Sparkles className="h-5 w-5 text-violet-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Intent Studio</h2>
            <p className="text-sm text-slate-400">What matters? Aether builds the intent block — no YAML editing required.</p>
          </div>
        </div>

        <label className="mb-4 block">
          <span className="mb-2 block text-xs font-medium uppercase tracking-wider text-slate-500">Workload name</span>
          <input
            value={workloadName}
            onChange={(e) => setWorkloadName(e.target.value)}
            className="w-full max-w-md rounded-xl border border-slate-700/80 bg-slate-950/60 px-4 py-2.5 text-sm text-slate-100 outline-none focus:border-violet-500/50"
            placeholder="my-app"
          />
        </label>

        <div className="mb-6">
          <span className="mb-3 block text-xs font-medium uppercase tracking-wider text-slate-500">What matters?</span>
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
                      : 'border-slate-800 bg-slate-900/50 text-slate-400 hover:border-slate-700'
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
          onClick={() => setGeneratedYaml(yaml)}
          className="rounded-xl bg-violet-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-violet-500"
          data-testid="intent-generate-button"
        >
          Generate Intent
        </button>

        {generatedYaml ? (
          <pre className="mt-6 overflow-x-auto rounded-2xl border border-slate-800/80 bg-slate-950/80 p-4 text-xs text-slate-300" data-testid="intent-generated-yaml">
            {generatedYaml}
          </pre>
        ) : null}
      </div>
    </section>
  );
}
