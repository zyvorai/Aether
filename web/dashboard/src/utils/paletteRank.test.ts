// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { groupByCategory, parseQuery, rankCommands, scoreCommand } from './paletteRank';
import { workloadCommands } from './paletteWorkloads';
import type { WorkloadResponse } from '../types/api';

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

describe('searching for a workload by name', () => {
  const workload = (name: string, status: string, extra: Partial<WorkloadResponse> = {}): WorkloadResponse => ({
    name, status, runtime: 'Podman', image: 'x', created_at: '', source: 'aether', ...extra,
  });
  const fleet = [workload('billing', 'running'), workload('web', 'stopped unexpectedly'), workload('web-api', 'running')];
  const commands = [
    ...fleet.flatMap((w) => workloadCommands(w, { gitopsConfigured: false })),
    cmd('Go to Alerts', 'alerts webhooks notifications'),
    cmd('Go to Test webhook', 'webhook test'),
    cmd('Open API explorer', 'openapi api routes'),
  ];
  const search = (query: string) => rankCommands(query, commands).map((c) => c.label);

  it('puts the workload first, then what you would do with it', () => {
    const found = search('web');
    expect(found.slice(0, 2)).toEqual(['Open workload: web', 'View logs: web']);
    expect(found).toContain('Why is web failing?');
    expect(found.indexOf('Open workload: web')).toBeLessThan(found.indexOf('Open workload: web-api'));
    expect(found.indexOf('Open workload: web-api')).toBeLessThan(found.indexOf('Go to Test webhook'));
    // One screen, not every page that exists once per workload.
    expect(found.filter((label) => label.endsWith(': web')).length).toBeLessThanOrEqual(7);
  });

  it('drops other workloads, hidden matches and actions that mean nothing for one', () => {
    const found = search('web');
    expect(found.filter((label) => label.includes('billing'))).toEqual([]);
    expect(found).not.toContain('Go to Alerts');
    for (const noise of ['Templates', 'Plugins', 'RBAC', 'OpenAPI', 'Compose', 'Platform', 'Backups', 'Secrets', 'Fleet view']) {
      expect(commands.filter((c) => c.label.startsWith(noise))).toEqual([]);
    }
  });

  it('finds the less common actions once they are named', () => {
    expect(search('web')).not.toContain('Policy check: web');
    expect(search('policy web')[0]).toBe('Policy check: web');
    expect(search('web sla')[0]).toBe('SLA compliance: web');
    expect(search('logs billing')[0]).toBe('View logs: billing');
    expect(search('logs')).toContain('View logs: billing');
  });

  it('offers only actions the workload can take now', () => {
    const labels = (w: WorkloadResponse) => workloadCommands(w, { gitopsConfigured: false }).map((c) => c.label);
    expect(labels(workload('a', 'running'))).toContain('Stop workload: a');
    expect(labels(workload('a', 'running'))).not.toContain('Start workload: a');
    expect(labels(workload('a', 'running'))).not.toContain('Why is a failing?');
    expect(labels(workload('a', 'stopped'))).toContain('Start workload: a');
    expect(labels(workload('a', 'stopped'))).not.toContain('Stop workload: a');
    expect(labels(workload('a', 'unknown'))).toEqual(expect.arrayContaining(['Start workload: a', 'Stop workload: a']));
    expect(labels(workload('a', 'running'))).not.toContain('Scale application: a');
    const discovered = labels(workload('c/n/app', 'running', { source: 'cluster', runtime: 'Kubernetes' }));
    expect(discovered).toContain('Scale application: c/n/app');
    expect(discovered).not.toContain('Stop workload: c/n/app');
    expect(labels(workload('a', 'running'))).not.toContain('GitOps sync: a');
  });

  it('leaves a page search alone when a longer phrase asks for the page', () => {
    expect(rankCommands('api explorer', [...commands, ...workloadCommands(workload('api', 'running'), { gitopsConfigured: false })]).map((c) => c.label)).toEqual(['Open API explorer']);
  });
});
