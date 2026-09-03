// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { Rocket, Wand2 } from 'lucide-react';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import GlassSection from './GlassSection';

function slugFromPrompt(prompt: string): string {
  const words = prompt
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, ' ')
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2);
  return words.join('-') || 'generated-app';
}

function generateSpec(prompt: string): string {
  const p = prompt.toLowerCase();
  const name = slugFromPrompt(prompt);
  const production = p.includes('production') || p.includes('prod');
  const ha = p.includes('ha') || p.includes('high availability') || production;
  const gpu = p.includes('gpu') || p.includes('cuda');
  const kafka = p.includes('kafka');
  const redis = p.includes('redis');
  const memory = kafka ? '4Gi' : gpu ? '8Gi' : '1Gi';
  const cpu = kafka ? '2000m' : gpu ? '4000m' : '500m';
  const storage = kafka ? '100Gi' : '10Gi';
  const replicas = ha ? 3 : 1;

  const intentLines = [
    production ? '  - high-availability' : null,
    p.includes('cost') || p.includes('cheap') ? '  - cost-optimized' : null,
    p.includes('latency') || p.includes('performance') ? '  - low-latency' : null,
    gpu ? '  - gpu' : null,
  ].filter(Boolean);

  const intentBlock =
    intentLines.length > 0 ? intentLines.join('\n') : '  - balanced';

  const extra =
    kafka || redis
      ? `
scaling:
  min_replicas: ${replicas}
  max_replicas: ${ha ? replicas * 2 : replicas}
monitoring:
  enabled: true
backup:
  enabled: ${production ? 'true' : 'false'}`
      : '';

  return `apiVersion: aether/v1
kind: Workload
metadata:
  name: ${name}
  owner: ai-studio
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
requirements:
  cpu: ${cpu}
  memory: ${memory}
  storage: ${storage}${gpu ? '\n  gpu:\n    vendor: nvidia\n    count: 1' : ''}
intent:
${intentBlock}
runtime:
  preferred: auto
  allow:
    - kube
    - podman
    - kubevirt
    - metal3${extra}
# Generated from: ${prompt.slice(0, 120)}
`;
}

export default function WorkloadDesignerPanel() {
  const navigate = useNavigate();
  const [prompt, setPrompt] = useState('');
  const [spec, setSpec] = useState('');

  const name = useMemo(() => slugFromPrompt(prompt), [prompt]);

  return (
    <GlassSection
      accent="purple"
      testId="workload-designer-panel"
      title="AI Workload Designer"
      subtitle="Describe the outcome — Aether generates workload spec, scaling, storage, and monitoring hooks."
      icon={<Wand2 className="h-5 w-5 text-brand" />}
    >
      <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={4}
          placeholder="Create Kafka cluster — 3 brokers, HA, 200k msgs/sec, production"
          className="glass-input"
          data-testid="workload-designer-prompt"
        />

        <div className="mt-4 flex flex-wrap gap-2">
          {[
            'Create Kafka cluster — 3 brokers, HA, production',
            'GPU inference service — low latency, 2 replicas',
            'Redis cache — cost optimized, kube',
          ].map((sample) => (
            <button
              key={sample}
              type="button"
              onClick={() => setPrompt(sample)}
              className="rounded-full border glass-divider px-3 py-1 text-xs text-muted hover:border-brand/30 hover:text-foreground"
            >
              {sample}
            </button>
          ))}
        </div>

        <button
          type="button"
          disabled={!prompt.trim()}
          onClick={() => setSpec(generateSpec(prompt))}
          className="mt-4 rounded-xl bg-brand px-4 py-2.5 text-sm font-medium text-white hover:bg-brand/90 disabled:opacity-50"
          data-testid="workload-designer-generate"
        >
          Generate spec
        </button>

        {spec ? (
          <>
            <pre className="glass-code-block-body mt-6 max-h-96 text-xs text-muted">
              {spec}
            </pre>
            <div className="mt-4 flex flex-wrap gap-2">
              <button
                type="button"
                onClick={() => navigate(pathWithQuery(viewToPath('workloads'), { deploy: '1', name }))}
                className="inline-flex items-center gap-2 rounded-xl bg-emerald-600 px-4 py-2.5 text-sm font-medium text-white hover:bg-emerald-500"
              >
                <Rocket className="h-4 w-4" />
                One-click deploy
              </button>
              <button
                type="button"
                onClick={() => navigate(pathWithQuery(viewToPath('editor'), { workload: name }))}
                className="rounded-xl border glass-divider px-4 py-2.5 text-sm text-muted hover:border-brand/40"
              >
                Open in editor
              </button>
            </div>
          </>
        ) : null}
    </GlassSection>
  );
}
