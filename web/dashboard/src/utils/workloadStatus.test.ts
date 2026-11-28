// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { countByCondition, needsAttention, statusVariant, workloadCondition } from './workloadStatus';
import { healthTone } from './k8sUx';
import { severityRank, severityTone } from './severity';
import { attentionVerdict, buildAttention, type AttentionSources } from './attention';
import type { WorkloadResponse } from '../types/api';

const workload = (name: string, status: string): WorkloadResponse => ({
  name,
  status,
  runtime: 'podman',
  image: 'x',
  created_at: '2026-01-01T00:00:00Z',
});

const sources = (workloads: WorkloadResponse[]): AttentionSources => ({
  workloads,
  events: [],
  secrets: [],
  health: { healthy: 0, degraded: 0, unhealthy: 0, unknown: 0, circuits_open: 0 },
  operator: { generated_at: '', workloads_checked: workloads.length, items: [] },
});

describe('observed workload status', () => {
  it('does not take "deployed" as evidence that something runs', () => {
    expect(workloadCondition('deployed (podman)')).toBe('unknown');
    expect(statusVariant('deployed (podman)')).not.toBe('green');
  });

  it('tells a requested stop from a failure on every surface', () => {
    expect(workloadCondition('stopped')).toBe('stopped');
    expect(needsAttention('stopped')).toBe(false);
    expect(statusVariant('stopped')).toBe('muted');
    expect(healthTone('stopped')).toBe('stopped');

    expect(workloadCondition('stopped unexpectedly')).toBe('failing');
    expect(needsAttention('stopped unexpectedly')).toBe(true);
    expect(statusVariant('stopped unexpectedly')).toBe('red');
    expect(healthTone('stopped unexpectedly')).toBe('critical');
    expect(severityTone('stopped unexpectedly')).toBe('danger');
  });

  it('shows not checked as neither healthy nor broken', () => {
    expect(workloadCondition('unknown')).toBe('unknown');
    expect(needsAttention('unknown')).toBe(false);
    expect(statusVariant('unknown')).toBe('muted');
    expect(healthTone('unknown')).toBe('unknown');
  });

  it('warns on degraded states', () => {
    for (const status of ['unhealthy', 'pending', 'not ready']) {
      expect(statusVariant(status)).toBe('yellow');
      expect(severityRank(status)).toBeLessThan(severityRank('running'));
    }
  });

  it('counts each workload once', () => {
    const counts = countByCondition(['running', 'stopped', 'stopped unexpectedly', 'unknown', 'unhealthy'].map((s) => ({ status: s })));
    expect(counts).toEqual({ running: 1, stopped: 1, failing: 1, unknown: 1, degraded: 1 });
  });

  it('never says all is well while a workload is unchecked', () => {
    const all = sources([workload('a', 'running'), workload('b', 'unknown')]);
    const verdict = attentionVerdict(buildAttention(all), all);
    expect(verdict.tone).toBe('unknown');
    expect(verdict.headline).toBe('1 workload has no current status');
  });

  it('lists an unexpected stop as critical and leaves a requested one alone', () => {
    const items = buildAttention(sources([workload('web', 'stopped unexpectedly'), workload('api', 'stopped')]));
    expect(items.map((i) => [i.id, i.severity, i.title])).toEqual([['workload-web', 'critical', 'web is stopped unexpectedly']]);
  });
});
