// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v5', () => {
  test('next actions API', async ({ request }) => {
    const res = await request.get('/api/command-center/next-actions');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.actions)).toBeTruthy();
    expect(data.actions.length).toBeGreaterThan(0);
  });

  test('unified agent registry API', async ({ request }) => {
    const res = await request.get('/api/intelligence/agents/status');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.agents.length).toBeGreaterThan(0);
  });

  test('healer execute dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/healer/execute', {
      data: { dry_run: true },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.dry_run).toBe(true);
    expect(Array.isArray(data.executed)).toBeTruthy();
    expect(Array.isArray(data.skipped)).toBeTruthy();
  });

  test('next actions panel on overview', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('command-center-next-actions')).toBeVisible({ timeout: 15_000 });
  });

  test('self-healing healer execute controls', async ({ page }) => {
    await page.goto('/observability');
    await expect(page.getByTestId('self-healing-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('healer-dry-run')).toBeVisible();
  });

  test('agent status dock uses registry', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto('/');
    await expect(page.getByTestId('agent-status-dock')).toBeVisible({ timeout: 15_000 });
  });
});
