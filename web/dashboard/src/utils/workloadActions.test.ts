// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { expectSuccess, isAetherManagedWorkload } from './workloadActions';
import type { WorkloadResponse } from '../types/api';

describe('workloadActions', () => {
  it('detects aether-managed vs cluster workloads', () => {
    expect(isAetherManagedWorkload({ name: 'web', runtime: 'kube', source: 'aether' } as WorkloadResponse)).toBe(true);
    expect(isAetherManagedWorkload({ name: 'c/n/app', runtime: 'kube', source: 'cluster' } as WorkloadResponse)).toBe(false);
  });
});

describe('action results', () => {
  it('turns a refused action into an error that says why', () => {
    expect(() => expectSuccess({ success: false, data: null, error: 'name in use\n' }, 'start')).toThrow('Start failed: name in use');
    expect(() => expectSuccess({ success: true, data: 'ok', error: null }, 'start')).not.toThrow();
  });
});
