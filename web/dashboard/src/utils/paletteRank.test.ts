// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { groupByCategory, parseQuery, scoreCommand } from './paletteRank';

const cmd = (label: string, searchText = '') => ({ label, searchText });
const best = (query: string, commands: Array<{ label: string; searchText: string }>) =>
  [...commands].sort((a, b) => scoreCommand(query, b) - scoreCommand(query, a))[0].label;

describe('command palette search', () => {
  it('reads scope prefixes', () => {
    expect(parseQuery('deploy')).toEqual({ scope: 'all', text: 'deploy' });
    expect(parseQuery('> theme')).toEqual({ scope: 'actions', text: 'theme' });
    expect(parseQuery('@web')).toEqual({ scope: 'workloads', text: 'web' });
    expect(parseQuery('/audit ')).toEqual({ scope: 'pages', text: 'audit' });
  });

  it('requires every word and accepts any order', () => {
    expect(scoreCommand('logs web', cmd('View logs: web'))).toBeGreaterThan(0);
    expect(scoreCommand('web logs', cmd('View logs: web'))).toBeGreaterThan(0);
    expect(scoreCommand('logs api', cmd('View logs: web'))).toBe(0);
  });

  it('prefers label matches over hidden search text', () => {
    expect(best('audit', [cmd('Go to Backups', 'restore audit trail'), cmd('Go to Audit Trail')])).toBe('Go to Audit Trail');
  });

  it('tolerates dropped letters but not scattered ones', () => {
    expect(scoreCommand('wrklds', cmd('Go to Workloads'))).toBeGreaterThan(0);
    expect(scoreCommand('gad', cmd('Generate SRE runbook for a deployment'))).toBe(0);
  });

  it('matches everything on an empty query', () => {
    expect(scoreCommand('', cmd('Anything'))).toBeGreaterThan(0);
  });

  it('keeps categories contiguous, best category first', () => {
    const ranked = [
      { id: 1, category: 'workload' },
      { id: 2, category: 'navigation' },
      { id: 3, category: 'workload' },
    ];
    expect(groupByCategory(ranked).map((c) => c.id)).toEqual([1, 3, 2]);
  });
});
