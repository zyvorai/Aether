// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

/** Cluster kinds that can open a kubectl exec shell against a related pod. */
export const SHELLABLE_CLUSTER_KINDS = [
  'Pod',
  'Deployment',
  'StatefulSet',
  'DaemonSet',
  'Job',
  'CronJob',
  'VirtualMachine',
  'VirtualMachineInstance',
] as const;

export type ShellableClusterKind = (typeof SHELLABLE_CLUSTER_KINDS)[number];

export function isShellableClusterKind(kind: string | null | undefined): boolean {
  return SHELLABLE_CLUSTER_KINDS.includes((kind ?? '') as ShellableClusterKind);
}

/** Kinds that have container/launcher logs via /api/cluster/logs. */
export const LOGS_CLUSTER_KINDS = [
  'Pod',
  'Deployment',
  'StatefulSet',
  'DaemonSet',
  'Service',
  'VirtualMachine',
  'VirtualMachineInstance',
  'HelmRelease',
  'Job',
  'CronJob',
] as const;

export function hasClusterLogs(kind: string | null | undefined): boolean {
  return (LOGS_CLUSTER_KINDS as readonly string[]).includes(kind ?? '');
}

/** Kinds that support kubectl rollout status/history/actions. */
export const ROLLOUT_CLUSTER_KINDS = ['Deployment', 'StatefulSet', 'DaemonSet'] as const;

export function isRolloutClusterKind(kind: string | null | undefined): boolean {
  return (ROLLOUT_CLUSTER_KINDS as readonly string[]).includes(kind ?? '');
}

/** Kinds that can start an Aether-managed port-forward session. */
export function canPortForwardClusterKind(kind: string | null | undefined): boolean {
  return isShellableClusterKind(kind) || kind === 'Service';
}

/** Short resource name from composite `cluster/namespace/name` discovery ids. */
export function clusterResourceName(workloadName: string): string {
  return workloadName.split('/').pop() ?? workloadName;
}

/** Phases that can accept kubectl exec (interactive shell). */
export function isExecReadyPhase(phase: string | null | undefined): boolean {
  const normalized = (phase ?? '').trim().toLowerCase();
  return normalized === 'running';
}

/** Pick a Running pod when available, otherwise the first related pod. */
export function pickExecPodName(
  kind: string | undefined,
  resourceName: string,
  pods: Array<{ name: string; phase: string }>,
): string {
  if (kind === 'Pod') return resourceName;
  const running = pods.find((pod) => isExecReadyPhase(pod.phase));
  return running?.name ?? pods[0]?.name ?? '';
}

/**
 * Prefer a pod that can actually accept exec. Returns empty when none are Ready
 * (e.g. Job pods that are Succeeded/Failed).
 */
export function pickExecReadyPodName(
  kind: string | undefined,
  resourceName: string,
  pods: Array<{ name: string; phase: string }>,
): string {
  if (kind === 'Pod') {
    if (pods.length === 0) return resourceName;
    const self = pods.find((pod) => pod.name === resourceName) ?? pods[0];
    if (!self) return resourceName;
    const phase = (self.phase ?? '').trim().toLowerCase();
    // Allow Unknown when pod detail could not be fully resolved (still try exec).
    if (phase === 'running' || phase === 'unknown' || phase === '') return self.name;
    return '';
  }
  return pods.find((pod) => isExecReadyPhase(pod.phase))?.name ?? '';
}

/** Honest message when Workloads Exec cannot open a live shell. */
export function execUnavailableMessage(
  kind: string | null | undefined,
  pods: Array<{ name: string; phase: string }>,
): string {
  if (pods.length === 0) {
    if (kind === 'Job' || kind === 'CronJob') {
      return '[aether] No pods found for this job. Open Logs for completed run output.\n';
    }
    if (kind === 'VirtualMachine' || kind === 'VirtualMachineInstance') {
      return '[aether] No pod available to exec into. Ensure the virt-launcher pod is running.\n';
    }
    return '[aether] No pod available to exec into.\n';
  }
  const phases = [...new Set(pods.map((pod) => pod.phase || 'Unknown'))].join(', ');
  return `[aether] No Running pod to exec into (found: ${phases}). Use Logs for completed containers.\n`;
}

export interface KubectlCommand {
  label: string;
  command: string;
}

export interface KubectlCommandInput {
  kind: string | null | undefined;
  namespace: string | null | undefined;
  /** Short resource name (already stripped of composite discovery prefix). */
  resourceName: string;
  /** Resolved pod name for logs/exec, when known. */
  podName?: string | null;
  /** Cluster/kube-context, added as `--context` when present. */
  context?: string | null;
}

/** Build the equivalent kubectl/virtctl commands so users can drop to a terminal. */
export function buildKubectlCommands(input: KubectlCommandInput): KubectlCommand[] {
  const { kind, resourceName } = input;
  if (!resourceName) return [];

  const ns = input.namespace ? ` -n ${input.namespace}` : '';
  const ctx = input.context ? ` --context ${input.context}` : '';
  const flags = `${ctx}${ns}`;
  const pod = input.podName || '';
  const isVm = kind === 'VirtualMachine' || kind === 'VirtualMachineInstance';

  const commands: KubectlCommand[] = [];

  if (isVm) {
    commands.push({ label: 'Console', command: `virtctl${flags} console ${resourceName}` });
    commands.push({ label: 'SSH', command: `virtctl${flags} ssh ${resourceName}` });
    if (pod) {
      commands.push({ label: 'Launcher logs', command: `kubectl${flags} logs ${pod} -f` });
    }
    commands.push({
      label: 'Describe',
      command: `kubectl${flags} describe ${(kind ?? 'vm').toLowerCase()} ${resourceName}`,
    });
    return commands;
  }

  const target = pod || resourceName;
  const kindLower = (kind ?? 'pod').toLowerCase();
  const logTarget = kind === 'Pod' ? target : `${kindLower}/${resourceName}`;

  commands.push({ label: 'Logs', command: `kubectl${flags} logs ${logTarget} -f --tail=200` });
  commands.push({ label: 'Exec', command: `kubectl${flags} exec -it ${target} -- /bin/sh` });
  commands.push({ label: 'Describe', command: `kubectl${flags} describe ${logTarget}` });
  commands.push({
    label: 'Port-forward',
    command: `kubectl${flags} port-forward ${logTarget} 8080:80`,
  });

  return commands;
}
