// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v15 — Platform & Ecosystem', () => {
  test('v1 briefing API', async ({ request }) => {
    const res = await request.get('/v1/intelligence/briefing');
    expect(res.ok()).toBeTruthy();
  });

  test('v1 threats API', async ({ request }) => {
    const res = await request.get('/v1/intelligence/threats');
    expect(res.ok()).toBeTruthy();
  });

  test('platform saas tenants API', async ({ request }) => {
    const res = await request.get('/api/intelligence/platform/saas-tenants');
    expect(res.ok()).toBeTruthy();
  });

  test('platform plugin marketplace API', async ({ request }) => {
    const res = await request.get('/api/intelligence/platform/plugin-marketplace');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.entries.length).toBeGreaterThan(0);
  });

  test('platform helm v2 API', async ({ request }) => {
    const res = await request.post('/api/intelligence/platform/helm-v2', {
      data: { goals: ['cost-optimized'], workload_name: 'demo' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.chart_yaml).toContain('apiVersion: v2');
  });

  test('platform public api manifest', async ({ request }) => {
    const res = await request.get('/api/intelligence/platform/public-api');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.version).toBe('v1');
  });

  test('platform community intents API', async ({ request }) => {
    const res = await request.get('/api/intelligence/platform/community-intents');
    expect(res.ok()).toBeTruthy();
  });

  test('platform autonomous sre API', async ({ request }) => {
    const res = await request.get('/api/intelligence/platform/autonomous-sre');
    expect(res.ok()).toBeTruthy();
  });

  test('platform autonomous sre execute API', async ({ request }) => {
    const res = await request.post('/api/intelligence/platform/autonomous-sre/execute', {
      data: { dry_run: true },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('ecosystem platform panel on platform page', async ({ page }) => {
    await page.goto('/platform');
    await expect(page.getByTestId('ecosystem-platform-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('ecosystem-saas-panel')).toBeVisible();
    await page.getByRole('button', { name: 'Public API' }).click();
    await expect(page.getByTestId('ecosystem-public-api-panel')).toBeVisible();
  });
});
