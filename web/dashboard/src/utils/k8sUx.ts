// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { WorkloadResponse } from '../types/api';

/** Treat a workload as a Kubernetes "Application" (managed or discovered). */
export function isK8sApplication(w: WorkloadResponse): boolean {
  const rt = w.runtime.toLowerCase();
  if (rt.includes('kube') || rt === 'kubernetes' || rt === 'kata') return true;
  if (w.cluster && w.namespace) return true;
  return false;
}

export function applicationLabel(w: WorkloadResponse): string {
  return w.name.split('/').pop() ?? w.name;
}

export function workspaceLabel(namespace: string | null | undefined): string {
  if (!namespace || namespace === 'default') return 'Default Workspace';
  return namespace
    .split('-')
    .map((p) => p.charAt(0).toUpperCase() + p.slice(1))
    .join(' ');
}

export interface FixRecommendation {
  title: string;
  summary: string;
  actions: { label: string; action: 'logs' | 'restart' | 'scale' | 'rollback' | 'editor' | 'clusters' | 'copilot' }[];
}

export function inferFixRecommendations(w: WorkloadResponse): FixRecommendation | null {
  const status = w.status.toLowerCase();
  const name = applicationLabel(w);

  if (status.includes('crashloop') || status.includes('crash')) {
    return {
      title: `${name} is repeatedly crashing`,
      summary:
        'Pods are exiting and restarting. Check logs for missing env vars, bad image tags, or failing readiness probes.',
      actions: [
        { label: 'Open Logs', action: 'logs' },
        { label: 'Restart', action: 'restart' },
        { label: 'Ask Copilot', action: 'copilot' },
      ],
    };
  }
  if (status.includes('imagepull') || status.includes('errimagepull')) {
    return {
      title: 'Cannot pull container image',
      summary:
        'The registry may be unreachable, the image name may be wrong, or pull secrets may be missing.',
      actions: [
        { label: 'Update Image', action: 'editor' },
        { label: 'Browse Cluster', action: 'clusters' },
      ],
    };
  }
  if (status.includes('pending')) {
    return {
      title: `${name} cannot be scheduled`,
      summary:
        'No node may have enough CPU/memory, or taints/tolerations may block placement.',
      actions: [
        { label: 'View Nodes', action: 'clusters' },
        { label: 'Ask Copilot', action: 'copilot' },
      ],
    };
  }
  if (status.includes('fail') || status.includes('error') || status.includes('unhealthy')) {
    return {
      title: `${name} needs attention`,
      summary: 'Health checks or deployment conditions report a problem.',
      actions: [
        { label: 'Open Logs', action: 'logs' },
        { label: 'Restart', action: 'restart' },
        { label: 'Rollback', action: 'rollback' },
      ],
    };
  }
  return null;
}

export function healthTone(status: string): 'healthy' | 'warning' | 'critical' | 'stopped' {
  const s = status.toLowerCase();
  if (s.includes('running') || s.includes('healthy') || s.includes('deployed')) return 'healthy';
  if (s.includes('stop') || s.includes('exit')) return 'stopped';
  if (s.includes('fail') || s.includes('error') || s.includes('crash')) return 'critical';
  return 'warning';
}
