// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { severityTone, sortBySeverity } from './severity';

describe('severity', () => {
  it('ranks lowercase runtime statuses like their capitalised forms', () => {
    expect(severityTone('failed')).toBe('danger');
    expect(severityTone('Failed')).toBe('danger');
    expect(severityTone('degraded')).toBe('warn');
    expect(severityTone('running')).toBe('neutral');
  });

  it('puts the worst first', () => {
    expect(sortBySeverity(['running', 'pending', 'failed'], (s) => s)).toEqual(['failed', 'pending', 'running']);
  });
});
