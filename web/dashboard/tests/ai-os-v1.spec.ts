// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('AI Infrastructure OS v1', () => {
  test('command center briefing API', async ({ request }) => {
    const res = await request.get('/api/command-center/briefing');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.greeting).toBeTruthy();
    expect(typeof data.fleet_health_pct).toBe('number');
  });

  test('command center briefing renders on overview', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('command-center-briefing')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('briefing-fleet-health')).toBeVisible();
  });

  test('runtime fabric page loads graph', async ({ page }) => {
    await page.goto('/fabric');
    await expect(page.getByTestId('runtime-fabric-graph')).toBeVisible({ timeout: 15_000 });
  });

  test('migration planner shell renders', async ({ page }) => {
    await page.goto('/migrations');
    await expect(page.getByTestId('migration-planner')).toBeVisible({ timeout: 15_000 });
  });

  test('fleet intelligence brief on fleet page', async ({ page }) => {
    await page.goto('/fleet');
    await expect(page.getByTestId('fleet-intelligence-brief')).toBeVisible({ timeout: 15_000 });
  });

  test('copilot rail visible on wide viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await expect(page.getByTestId('copilot-rail')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('copilot-rail-input')).toBeVisible();
  });

  test('fleet root cause API', async ({ request }) => {
    const res = await request.get('/api/copilot/troubleshoot/fleet');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.diagnoses)).toBeTruthy();
  });

  test('observability root cause panel', async ({ page }) => {
    await page.goto('/observability');
    await expect(page.getByTestId('fleet-root-cause-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('ai studio intent tab', async ({ page }) => {
    await page.goto('/ai?tab=intent');
    await expect(page.getByTestId('intent-studio-panel')).toBeVisible({ timeout: 15_000 });
  });
});
