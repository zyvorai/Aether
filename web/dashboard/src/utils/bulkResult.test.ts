// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { bulkSummary } from './bulkResult';

describe('bulk workload actions', () => {
  it('reports success only when every action succeeded', () => {
    expect(bulkSummary('stop', [{ name: 'a', success: true }, { name: 'b', success: true }])).toEqual({
      message: 'Bulk stop succeeded for 2 workload(s)',
      type: 'success',
    });
  });

  it('names what failed and why', () => {
    const result = bulkSummary('start', [
      { name: 'a', success: true },
      { name: 'b', success: false, error: 'name in use\n' },
    ]);
    expect(result.type).toBe('error');
    expect(result.message).toBe('Bulk start failed for 1 of 2: b. b: name in use');
  });
});
