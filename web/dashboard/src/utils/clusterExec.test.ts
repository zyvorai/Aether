// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { describe, expect, it } from 'vitest';
import { pickExecPodName } from './clusterExec';

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
