// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { describe, expect, it } from 'vitest';
import {
  buildKubectlCommands,
  clusterResourceName,
  hasClusterLogs,
  isShellableClusterKind,
  pickExecPodName,
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

describe('clusterExec helpers', () => {
  it('strips composite discovery names', () => {
    expect(clusterResourceName('active-client/default/playwright-nginx')).toBe('playwright-nginx');
  });

  it('identifies shellable and loggable kinds', () => {
    expect(isShellableClusterKind('Pod')).toBe(true);
    expect(isShellableClusterKind('Job')).toBe(false);
    expect(hasClusterLogs('Job')).toBe(true);
    expect(hasClusterLogs('ConfigMap')).toBe(false);
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
