// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  buildKubectlCommands,
  clusterResourceName,
  execUnavailableMessage,
  hasClusterLogs,
  isExecReadyPhase,
  isRolloutClusterKind,
  isShellableClusterKind,
  canPortForwardClusterKind,
  pickExecPodName,
  pickExecReadyPodName,
} from './clusterExec';

describe('pickExecPodName', () => {
  it('uses the resource name for Pod kind', () => {
    expect(pickExecPodName('Pod', 'nginx', [{ name: 'other', phase: 'Running' }])).toBe('nginx');
  });

  it('prefers a Running related pod for deployments and VMs', () => {
    expect(
      pickExecPodName('VirtualMachine', 'demo-vm', [
        { name: 'virt-launcher-pending', phase: 'Pending' },
        { name: 'virt-launcher-running', phase: 'Running' },
      ]),
    ).toBe('virt-launcher-running');
  });

  it('falls back to the first related pod when none are Running', () => {
    expect(
      pickExecPodName('Deployment', 'web', [
        { name: 'web-abc', phase: 'Pending' },
        { name: 'web-def', phase: 'Failed' },
      ]),
    ).toBe('web-abc');
  });

  it('returns empty string when there are no related pods', () => {
    expect(pickExecPodName('VirtualMachineInstance', 'demo-vmi', [])).toBe('');
  });
});

describe('pickExecReadyPodName', () => {
  it('only returns Running pods for Jobs and Deployments', () => {
    expect(
      pickExecReadyPodName('Job', 'batch', [
        { name: 'batch-1', phase: 'Succeeded' },
        { name: 'batch-2', phase: 'Failed' },
      ]),
    ).toBe('');
    expect(
      pickExecReadyPodName('Job', 'batch', [
        { name: 'batch-1', phase: 'Succeeded' },
        { name: 'batch-2', phase: 'Running' },
      ]),
    ).toBe('batch-2');
  });

  it('rejects completed Pods but allows Unknown detail gaps', () => {
    expect(pickExecReadyPodName('Pod', 'nginx', [{ name: 'nginx', phase: 'Succeeded' }])).toBe('');
    expect(pickExecReadyPodName('Pod', 'nginx', [{ name: 'nginx', phase: 'Unknown' }])).toBe('nginx');
  });
});

describe('execUnavailableMessage', () => {
  it('points Jobs at Logs when pods are terminal', () => {
    expect(isExecReadyPhase('Succeeded')).toBe(false);
    expect(isExecReadyPhase('Running')).toBe(true);
    expect(
      execUnavailableMessage('Job', [{ name: 'j-1', phase: 'Succeeded' }]),
    ).toContain('No Running pod');
    expect(execUnavailableMessage('Job', [])).toContain('Open Logs');
  });
});

describe('clusterExec helpers', () => {
  it('strips composite discovery names', () => {
    expect(clusterResourceName('active-client/default/playwright-nginx')).toBe('playwright-nginx');
  });

  it('identifies shellable and loggable kinds', () => {
    expect(isShellableClusterKind('Pod')).toBe(true);
    expect(isShellableClusterKind('Job')).toBe(true);
    expect(isShellableClusterKind('CronJob')).toBe(true);
    expect(isShellableClusterKind('ConfigMap')).toBe(false);
    expect(hasClusterLogs('Job')).toBe(true);
    expect(hasClusterLogs('ConfigMap')).toBe(false);
    expect(isRolloutClusterKind('Deployment')).toBe(true);
    expect(isRolloutClusterKind('Pod')).toBe(false);
    expect(canPortForwardClusterKind('Service')).toBe(true);
    expect(canPortForwardClusterKind('ConfigMap')).toBe(false);
  });
});

describe('buildKubectlCommands', () => {
  it('builds pod logs/exec with namespace and context flags', () => {
    const cmds = buildKubectlCommands({
      kind: 'Pod',
      namespace: 'web',
      resourceName: 'nginx-abc',
      podName: 'nginx-abc',
      context: 'prod',
    });
    const byLabel = Object.fromEntries(cmds.map((c) => [c.label, c.command]));
    expect(byLabel.Logs).toBe('kubectl --context prod -n web logs nginx-abc -f --tail=200');
    expect(byLabel.Exec).toBe('kubectl --context prod -n web exec -it nginx-abc -- /bin/sh');
  });

  it('targets the workload for deployment logs but the pod for exec', () => {
    const cmds = buildKubectlCommands({
      kind: 'Deployment',
      namespace: 'default',
      resourceName: 'web',
      podName: 'web-7d9-xyz',
    });
    const byLabel = Object.fromEntries(cmds.map((c) => [c.label, c.command]));
    expect(byLabel.Logs).toBe('kubectl -n default logs deployment/web -f --tail=200');
    expect(byLabel.Exec).toBe('kubectl -n default exec -it web-7d9-xyz -- /bin/sh');
  });

  it('uses virtctl for virtual machines', () => {
    const cmds = buildKubectlCommands({
      kind: 'VirtualMachine',
      namespace: 'vms',
      resourceName: 'demo-vm',
      podName: 'virt-launcher-demo-vm-abc',
    });
    const labels = cmds.map((c) => c.label);
    expect(labels).toContain('Console');
    expect(labels).toContain('SSH');
    expect(cmds[0].command).toBe('virtctl -n vms console demo-vm');
  });

  it('returns nothing without a resource name', () => {
    expect(buildKubectlCommands({ kind: 'Pod', namespace: 'x', resourceName: '' })).toEqual([]);
  });
});
