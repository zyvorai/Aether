// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import {
  attentionTarget,
  buildPreviewAction,
  formatMinutes,
  formatReadyPct,
  formatSeconds,
  friendlyError,
  parseTab,
  queryString,
  workloadPath,
} from './operator';
import type { AttentionItem } from '../types/operator';

const item = (kind: string, title: string, workload: string | null = 'web'): AttentionItem => ({
  level: 'warning',
  workload,
  kind,
  title,
  detail: '',
  next: null,
});

describe('operator helpers', () => {
  it('falls back to the attention tab', () => {
    expect(parseTab('report')).toBe('report');
    expect(parseTab('nonsense')).toBe('attention');
    expect(parseTab('')).toBe('attention');
  });

  it('shows missing measurements as missing', () => {
    expect(formatReadyPct(null)).toBe('no data');
    expect(formatReadyPct(0)).toBe('0.00%');
    expect(formatMinutes(null)).toBe('no recovered incident');
    expect(formatMinutes(0)).toBe('0.0 min');
  });

  it('formats durations', () => {
    expect(formatSeconds(-5)).toBe('0s');
    expect(formatSeconds(75)).toBe('1m 15s');
    expect(formatSeconds(3660)).toBe('1h 1m');
    expect(formatSeconds(90000)).toBe('1d 1h');
  });

  it('builds queries and escapes names', () => {
    expect(queryString({ since: '24h', workload: '', limit: undefined })).toBe('?since=24h');
    expect(queryString({})).toBe('');
    expect(workloadPath('a/b', 'explain')).toBe('/workloads/a%2Fb/explain');
  });

  it('links attention items to the right tab', () => {
    expect(attentionTarget(item('open-incident', 'Incident 12 is open'))).toEqual({ tab: 'incidents', incident: '12' });
    expect(attentionTarget(item('failed-operation', '1 failed'))).toEqual({ tab: 'changes', workload: 'web' });
    expect(attentionTarget(item('unapplied-spec', 'x'))).toEqual({ tab: 'workload', workload: 'web' });
    expect(attentionTarget(item('audit-integrity', 'x', null))).toBeNull();
  });

  it('validates preview input before calling the server', () => {
    const form = { yaml: '', target: '', revision: '' };
    expect(buildPreviewAction({ ...form, kind: 'delete' })).toEqual({ action: { action: 'delete' } });
    expect(buildPreviewAction({ ...form, kind: 'update' })).toHaveProperty('error');
    expect(buildPreviewAction({ ...form, kind: 'migrate' })).toHaveProperty('error');
    expect(buildPreviewAction({ ...form, kind: 'rollback' })).toEqual({ action: { action: 'rollback' } });
    expect(buildPreviewAction({ ...form, kind: 'rollback', revision: '3' })).toEqual({
      action: { action: 'rollback', revision: 3 },
    });
    for (const revision of ['0', '-1', '1.5', 'two']) {
      expect(buildPreviewAction({ ...form, kind: 'rollback', revision })).toHaveProperty('error');
    }
  });

  it('explains a refused request', () => {
    expect(friendlyError('SyntaxError: Unexpected end of JSON input')).toContain('role');
    expect(friendlyError('on hold')).toBe('on hold');
    expect(friendlyError(null)).toBe('The request failed.');
  });
});
