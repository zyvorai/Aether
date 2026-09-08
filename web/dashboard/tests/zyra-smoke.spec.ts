// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('Zyra smoke', () => {
  test('zyra agents API', async ({ request }) => {
    const res = await request.get('/api/zyra/agents');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.data)).toBe(true);
  });

  test('zyra providers status', async ({ request }) => {
    const res = await request.get('/api/zyra/providers/status');
    expect(res.ok()).toBeTruthy();
  });

  test('zyra insights', async ({ request }) => {
    const res = await request.get('/api/zyra/insights');
    expect(res.ok()).toBeTruthy();
  });

  test('deprecated copilot chat alias', async ({ request }) => {
    const res = await request.post('/api/copilot/chat', {
      data: { message: 'list workloads' },
      failOnStatusCode: false,
    });
    expect(res.headers()['deprecation']).toBeDefined();
    // 200 when LLM reachable; 5xx when provider rate-limits or is offline
    expect([200, 500, 502, 503, 504]).toContain(res.status());
  });

  test('zyra marketplace', async ({ request }) => {
    const res = await request.get('/api/zyra/marketplace');
    expect(res.ok()).toBeTruthy();
  });
});
