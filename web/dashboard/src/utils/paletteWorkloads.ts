// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * What the command palette offers for a workload: only things that make sense
 * for that workload as it is now.
 */

import type { AppView, WorkloadResponse } from '../types/api';
import { hasClusterLogs } from './clusterExec';
import { isK8sApplication } from './k8sUx';
import type { WorkloadTier } from './paletteRank';
import { isAetherManaged } from './workloadFilters';
import { needsAttention, workloadCondition } from './workloadStatus';

export interface WorkloadCommand {
  id: string;
  label: string;
  searchText: string;
  category: 'workload' | 'workload-action';
  workload: { name: string; tier: WorkloadTier };
  workloadTab?: string;
  /** Shown beside the entry that opens the workload. */
  status?: string;
  to?: { view: AppView; params: Record<string, string>; select?: boolean };
  action?: 'start' | 'stop';
}

export function workloadCommands(w: WorkloadResponse, { gitopsConfigured }: { gitopsConfigured: boolean }): WorkloadCommand[] {
  const name = w.name;
  const managed = isAetherManaged(w);
  const app = isK8sApplication(w);
  const condition = workloadCondition(w.status);
  const commands: WorkloadCommand[] = [];

  const add = (
    tier: WorkloadTier,
    id: string,
    label: string,
    keywords: string,
    rest: Pick<WorkloadCommand, 'to' | 'action' | 'workloadTab' | 'status'>,
  ) =>
    commands.push({
      id,
      label,
      searchText: `${keywords} ${name}`,
      category: tier === 'self' ? 'workload' : 'workload-action',
      workload: { name, tier },
      ...rest,
    });
  const page = (view: AppView, params: Record<string, string> = {}) => ({ to: { view, params: { workload: name, ...params } } });
  const tab = (id: string) => ({ workloadTab: id, to: { view: 'workloads' as AppView, params: { workload: name, tab: id }, select: true } });

  // Order here is the order shown when only the name is typed.
  add('self', `workload-${name}`, `Open workload: ${name}`, 'workload', { status: w.status, to: { view: 'workloads', params: { workload: name }, select: true } });
  if (app) add('self', `application-${name}`, `Open application: ${name}`, 'application app kubernetes', page('applications'));

  if (managed || hasClusterLogs(w.kind ?? undefined)) add('primary', `workload-${name}-logs`, `View logs: ${name}`, 'logs workload', tab('logs'));
  if (needsAttention(w.status)) {
    add('primary', `workload-${name}-why-failing`, `Why is ${name} failing?`, 'why failing troubleshoot diagnose', page('zyra', { q: `Why is ${name} failing?` }));
  }
  if (managed && condition !== 'running' && condition !== 'degraded') {
    add('primary', `workload-${name}-start`, `Start workload: ${name}`, 'start run', { action: 'start' });
  }
  if (managed && condition !== 'stopped') add('primary', `workload-${name}-stop`, `Stop workload: ${name}`, 'stop halt', { action: 'stop' });
  add('primary', `workload-${name}-events`, `View events: ${name}`, 'events workload', { workloadTab: 'events', ...page('events') });
  if (managed) add('primary', `workload-${name}-drift`, `Check drift: ${name}`, 'drift reconcile workload', tab('drift'));
  add('primary', `workload-${name}-health`, `Health monitor: ${name}`, 'health monitor liveness workload', page('health'));

  // Offered when the action is named too, as in "policy web".
  const more = (id: string, label: string, keywords: string, view: AppView, params: Record<string, string> = {}) =>
    add('secondary', `workload-${name}-${id}`, `${label}: ${name}`, keywords, page(view, params));
  more('audit', 'Audit trail', 'audit trail history', 'audit');
  more('alerts', 'Alert rules', 'alerts webhooks', 'alerts');
  more('metrics', 'Metrics', 'metrics grafana prometheus', 'metrics');
  more('sla', 'SLA compliance', 'sla uptime compliance', 'sla');
  more('policy', 'Policy check', 'policy opa validate', 'policy');
  more('cost', 'Cost estimate', 'cost chargeback', 'cost');
  more('intelligence', 'Intelligence', 'intelligence predictions threats', 'intelligence', { tab: 'predictions' });
  more('zyra', 'Ask Zyra about', 'copilot ask health', 'zyra', { q: `What is the state of ${name}?` });
  if (managed) {
    more('deps', 'Dependencies', 'dependencies graph startup', 'deps');
    more('editor', 'Visual editor', 'editor visual yaml spec', 'editor');
    add('secondary', `workload-${name}-scoring`, `Open scoring: ${name}`, 'scoring ai intent', tab('scoring'));
    more('scheduler', 'Scheduler', 'scheduler placement', 'scheduler');
    more('affinity', 'Runtime affinity', 'affinity runtime class', 'affinity');
    if (gitopsConfigured) more('gitops', 'GitOps sync', 'gitops sync reconcile', 'gitops');
  }
  if (app) add('secondary', `workload-${name}-scale`, `Scale application: ${name}`, 'scale replicas application', page('applications', { tab: 'overview' }));
  return commands;
}
