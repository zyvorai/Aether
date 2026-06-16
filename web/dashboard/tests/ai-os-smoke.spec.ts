// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS smoke', () => {
  test('fabric topology API', async ({ request }) => {
    const res = await request.get('/api/intelligence/fabric/topology');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.nodes)).toBeTruthy();
    expect(Array.isArray(data.edges)).toBeTruthy();
  });

  test('command center is narrative home', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await expect(page.getByTestId('command-center-briefing')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('command-center-next-actions')).toBeVisible();
    await expect(page.getByTestId('overview-legacy-details')).not.toBeVisible();
  });

  test('classic nav preference in settings', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByTestId('nav-preferences-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('knowledge graph on fabric page', async ({ page }) => {
    await page.goto('/fabric');
    await expect(page.getByTestId('knowledge-graph-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('runtime advisor default on AI studio', async ({ page }) => {
    await page.goto('/ai');
    await expect(page.getByTestId('runtime-advisor-panel')).toBeVisible({ timeout: 15_000 });
  });
});
