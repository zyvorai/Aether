// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v3', () => {
  test('autonomy status API', async ({ request }) => {
    const res = await request.get('/api/intelligence/autonomy/status');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.effective_policy).toBeTruthy();
  });

  test('knowledge graph API', async ({ request }) => {
    const res = await request.get('/api/intelligence/knowledge-graph');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.nodes)).toBeTruthy();
    expect(data.stats).toBeTruthy();
  });

  test('healer preview API', async ({ request }) => {
    const res = await request.get('/api/intelligence/healer/preview');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.would_execute)).toBeTruthy();
  });

  test('autonomous mode panel on settings', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByTestId('autonomous-mode-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('self-healing panel on observability', async ({ page }) => {
    await page.goto('/observability');
    await expect(page.getByTestId('self-healing-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('knowledge graph on labs', async ({ page }) => {
    await page.goto('/labs');
    await expect(page.getByTestId('knowledge-graph-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('agent status dock on wide viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await expect(page.getByTestId('agent-status-dock')).toBeVisible({ timeout: 15_000 });
  });
});
