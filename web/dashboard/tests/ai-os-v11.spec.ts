// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v11 — macOS Native OS', () => {
  test('tray sparkline API', async ({ request }) => {
    const res = await request.get('/api/intelligence/macos/tray-sparkline');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.sparkline.length).toBeGreaterThan(0);
  });

  test('dock badge API', async ({ request }) => {
    const res = await request.get('/api/intelligence/macos/dock-badge');
    expect(res.ok()).toBeTruthy();
  });

  test('live activity API', async ({ request }) => {
    const res = await request.get('/api/intelligence/macos/live-activity');
    expect(res.ok()).toBeTruthy();
  });

  test('spotlight index API', async ({ request }) => {
    const res = await request.get('/api/intelligence/macos/spotlight');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.items.length).toBeGreaterThan(0);
  });

  test('shortcuts manifest API', async ({ request }) => {
    const res = await request.get('/api/intelligence/macos/shortcuts');
    expect(res.ok()).toBeTruthy();
  });

  test('offline cache API', async ({ request }) => {
    const res = await request.get('/api/intelligence/macos/offline-cache');
    expect(res.ok()).toBeTruthy();
  });

  test('universal link resolve API', async ({ request }) => {
    const res = await request.get(
      '/api/intelligence/macos/universal-links/resolve?url=aether://workloads/api',
    );
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.view).toBe('workloads');
  });

  test('release pipeline API', async ({ request }) => {
    const res = await request.get('/api/intelligence/macos/release-pipeline');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.steps.length).toBe(4);
  });

  test('macOS platform panel on settings', async ({ page }) => {
    await page.goto('/settings');
    await expect(page.getByTestId('macos-platform-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('tray-sparkline-panel')).toBeVisible();
    await expect(page.getByTestId('universal-links-panel')).toBeVisible();
  });
});
