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

describe('attention lists each problem once', () => {
  const incident = {
    level: 'critical' as const,
    kind: 'open-incident',
    title: 'Incident 1 is open',
    workload: 'web',
    detail: 'stopped for 2m 0s, 4 failed checks, not acknowledged',
    next: null,
  };
  const opened: Event = {
    ...event('Incident opened', 'critical', 'web'),
    message: 'incident 1: web is not ready (stopped) after 2 consecutive checks',
  };
  const queue = (items: NonNullable<AttentionSources['operator']>['items']) => ({ generated_at: '', workloads_checked: 1, items });

  for (const [when, workloads] of [
    ['the workload is listed as down', [workload('web', 'stopped unexpectedly')]],
    ['the workload status is unknown', [workload('web', 'unknown')]],
    ['the workload list did not load', null],
  ] as const) {
    it(`shows one incident once when ${when}`, () => {
      const items = buildAttention({
        ...healthy,
        workloads: workloads ? [...workloads] : null,
        events: [opened],
        operator: queue([incident]),
      });
      expect(items).toHaveLength(1);
      expect(items[0].severity).toBe('critical');
      const text = `${items[0].title} ${items[0].detail}`;
      expect(text).toContain('Incident 1 is open');
      expect(text).toContain('4 failed checks');
      expect(text).not.toContain('Incident opened');
    });
  }

  it('folds a failed operation event into the server count of failures', () => {
    const items = buildAttention({
      ...healthy,
      events: [{ ...event('stop failed', 'error', 'web'), message: 'stop failed on podman: boom' }],
      operator: queue([
        { level: 'warning', kind: 'failed-operation', title: '1 failed operation(s) in 24h', workload: 'web', detail: 'latest: stop failed on podman: boom', next: null },
      ]),
    });
    expect(items.map((i) => i.title)).toEqual(['web: 1 failed operation(s) in 24h']);
    expect(items[0].severity).toBe('critical');
  });

  it('keeps other events and other workloads apart', () => {
    const items = buildAttention({
      ...healthy,
      events: [opened, event('Disk filling', 'warning', 'web'), { ...opened, workload: 'api' }],
      operator: queue([incident]),
    });
    expect(items.map((i) => i.title).sort()).toEqual(['Disk filling', 'Incident opened', 'web: Incident 1 is open']);
  });

  it('keeps the incident event when the server queue did not load', () => {
    const items = buildAttention({ ...healthy, events: [opened], operator: null });
    expect(items.map((i) => i.title)).toEqual(['Incident opened']);
  });
});
