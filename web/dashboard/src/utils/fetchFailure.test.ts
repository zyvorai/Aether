// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { describeFetchFailure } from './fetchFailure';

describe('describeFetchFailure', () => {
  it('reports a request that got no answer', () => {
    expect(describeFetchFailure('/workloads')).toBe('GET /api/workloads: no response from the server');
  });

  it('includes the status and the server message', () => {
    expect(describeFetchFailure('/events', 503, 'unavailable')).toBe('GET /api/events: HTTP 503, unavailable');
    expect(describeFetchFailure('/events', 502, '  ')).toBe('GET /api/events: HTTP 502, unreadable response');
  });

  it('cuts very long server messages', () => {
    expect(describeFetchFailure('/events', 500, 'x'.repeat(900)).length).toBeLessThan(300);
  });
});
