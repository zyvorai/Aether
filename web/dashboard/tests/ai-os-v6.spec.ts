// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v6', () => {
  test('critical notifications API', async ({ request }) => {
    const res = await request.get('/api/command-center/notifications');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.notifications)).toBeTruthy();
  });

  test('evolution execute dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/evolution/execute', {
      data: { dry_run: true, max_actions: 3 },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.dry_run).toBe(true);
  });

  test('gitops agent plan API', async ({ request }) => {
    const res = await request.get('/api/intelligence/gitops/agent/plan');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.planned_actions)).toBeTruthy();
  });

  test('cost optimize apply dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/cost-optimize/apply', { data: { dry_run: true } });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.dry_run).toBe(true);
  });

  test('security remediate dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/security/remediate', { data: { dry_run: true } });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.applied)).toBeTruthy();
  });

  test('capacity scale suggestions API', async ({ request }) => {
    const res = await request.get('/api/intelligence/capacity/scale-suggestions');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.suggestions)).toBeTruthy();
  });

  test('capacity scale panel on observability', async ({ page }) => {
    await page.goto('/observability');
    await expect(page.getByTestId('capacity-scale-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('gitops agent panel', async ({ page }) => {
    await page.goto('/gitops');
    await expect(page.getByTestId('gitops-agent-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('evolution dry-run on migrations', async ({ page }) => {
    await page.goto('/migrations');
    await expect(page.getByTestId('evolution-dry-run')).toBeVisible({ timeout: 15_000 });
  });
});
