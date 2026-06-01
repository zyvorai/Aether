import { test, expect } from '@playwright/test';

test.describe('Zeus smoke', () => {
  test('zeus agents API', async ({ request }) => {
    const res = await request.get('/api/zeus/agents');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.success).toBe(true);
    expect(Array.isArray(body.data)).toBe(true);
  });

  test('zeus providers status', async ({ request }) => {
    const res = await request.get('/api/zeus/providers/status');
    expect(res.ok()).toBeTruthy();
  });

  test('zeus insights', async ({ request }) => {
    const res = await request.get('/api/zeus/insights');
    expect(res.ok()).toBeTruthy();
  });

  test('deprecated copilot chat alias', async ({ request }) => {
    const res = await request.post('/api/copilot/chat', {
      data: { message: 'list workloads' },
    });
    expect(res.ok()).toBeTruthy();
    expect(res.headers()['deprecation']).toBeDefined();
  });

  test('zeus marketplace', async ({ request }) => {
    const res = await request.get('/api/zeus/marketplace');
    expect(res.ok()).toBeTruthy();
  });
});
