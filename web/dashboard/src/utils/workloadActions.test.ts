// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { describe, expect, it } from 'vitest';
import { isAetherManagedWorkload } from './workloadActions';
import type { WorkloadResponse } from '../types/api';

describe('workloadActions', () => {
  it('detects aether-managed vs cluster workloads', () => {
    expect(isAetherManagedWorkload({ name: 'web', runtime: 'kube', source: 'aether' } as WorkloadResponse)).toBe(true);
    expect(isAetherManagedWorkload({ name: 'c/n/app', runtime: 'kube', source: 'cluster' } as WorkloadResponse)).toBe(false);
  });
});
