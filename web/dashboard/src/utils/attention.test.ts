// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { attentionVerdict, buildAttention, type AttentionSources } from './attention';
import { workloadCondition } from './workloadStatus';
import type { Event, WorkloadResponse } from '../types/api';

const workload = (name: string, status: string): WorkloadResponse => ({
  name,
  status,
  runtime: 'podman',
  image: 'x',
  created_at: '2026-01-01T00:00:00Z',
});

const event = (title: string, severity: string, workloadName: string | null, acknowledged = false): Event => ({
  timestamp: '2026-01-01T00:00:00Z',
  severity,
  category: 'workload',
  title,
  message: `${title} message`,
  source: 'test',
  workload: workloadName,
  acknowledged,
});

const healthy: AttentionSources = {
  workloads: [workload('a', 'running'), workload('b', 'stopped')],
  events: [],
  secrets: [],
  health: { healthy: 1, degraded: 0, unhealthy: 0, unknown: 0, circuits_open: 0 },
  operator: { generated_at: '', workloads_checked: 2, items: [] },
};

describe('workloadCondition', () => {
  it('reads runtime status strings regardless of case', () => {
    expect(workloadCondition('Running')).toBe('running');
    expect(workloadCondition('deployed (kubernetes)')).toBe('unknown');
    expect(workloadCondition('CrashLoopBackOff')).toBe('failing');
    expect(workloadCondition('NotReady')).toBe('degraded');
    expect(workloadCondition('exited')).toBe('stopped');
    expect(workloadCondition('something-new')).toBe('unknown');
    expect(workloadCondition(undefined)).toBe('unknown');
  });
});

describe('attention', () => {
  it('says nothing needs attention only when every source was checked', () => {
    expect(buildAttention(healthy)).toEqual([]);
    expect(attentionVerdict([], healthy)).toMatchObject({ tone: 'clear', unchecked: [] });
  });

  it('never reads a failed source as healthy', () => {
    const sources = { ...healthy, events: null };
    const verdict = attentionVerdict(buildAttention(sources), sources);
    expect(verdict.tone).toBe('unknown');
    expect(verdict.unchecked).toEqual(['events']);
  });

  it('merges a workload with its open events and puts critical items first', () => {
    const items = buildAttention({
      workloads: [workload('slow', 'degraded'), workload('down', 'failed')],
      events: [
        event('Container exited', 'CRITICAL', 'down'),
        event('Probe failing', 'WARN', 'down'),
        event('Disk filling', 'warning', null),
        event('Disk filling', 'warning', null),
        event('Old news', 'critical', null, true),
        event('Deployed', 'info', 'slow'),
      ],
      secrets: [{ name: 's', namespace: 'default', key_count: 1, created_at: '', updated_at: '', needs_rotation: true }],
      health: null,
    });
    expect(items.map((i) => i.id)).toEqual([
      'workload-down',
      'workload-slow',
      'event-Disk filling|',
      'secrets-rotation',
    ]);
    expect(items[0].detail).toContain('+1 more open event');
    expect(items[2].title).toBe('Disk filling (2 times)');
  });

  it('does not list a rotation reminder twice', () => {
    const items = buildAttention({
      ...healthy,
      events: [event('Secret rotation due', 'warning', null)].map((e) => ({ ...e, message: 'db-password is due for rotation' })),
      secrets: [{ name: 'db-password', namespace: 'default', key_count: 1, created_at: '', updated_at: '', needs_rotation: true }],
    });
    expect(items.map((i) => i.id)).toEqual(['secrets-rotation']);
  });

  it('adds the server queue: one row per workload, no informational noise', () => {
    const entry = (level: 'info' | 'warning' | 'critical', kind: string, title: string, workloadName: string | null) => ({
      level, kind, title, workload: workloadName, detail: `${title} detail`, next: null,
    });
    const items = buildAttention({
      ...healthy,
      workloads: [workload('down', 'degraded')],
      operator: {
        generated_at: '',
        workloads_checked: 2,
        items: [
          entry('critical', 'open-incident', 'Incident 3 is open', 'down'),
          entry('warning', 'failed-operation', '1 failed operation(s) in 24h', 'other'),
          entry('info', 'on-hold', 'On hold', 'other'),
          entry('critical', 'audit-integrity', '1 audit event(s) fail the integrity check', null),
        ],
      },
    });
    expect(items.map((i) => [i.view, i.severity])).toEqual([
      ['workloads', 'critical'],
      ['audit', 'critical'],
      ['operations', 'warning'],
    ]);
    expect(items[0].detail).toContain('Incident 3 is open');
    expect(items[2].query).toEqual({ tab: 'changes', workload: 'other' });
  });

  it('reports an unloaded server queue as unchecked', () => {
    const sources = { ...healthy, operator: null };
    expect(attentionVerdict(buildAttention(sources), sources)).toMatchObject({ tone: 'unknown', unchecked: ['recorded operations'] });
  });

  it('reports open circuit breakers', () => {
    const items = buildAttention({ ...healthy, health: { healthy: 0, degraded: 0, unhealthy: 0, unknown: 0, circuits_open: 2 } });
    expect(items[0]).toMatchObject({ id: 'circuits-open', severity: 'critical', title: '2 circuit breakers open' });
  });
});
