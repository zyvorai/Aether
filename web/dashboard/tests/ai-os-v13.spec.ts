// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('AI Infrastructure OS v13 — FinOps & Cost', () => {
  test('finops chargeback API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/chargeback');
    expect(res.ok()).toBeTruthy();
  });

  test('finops spot advisor API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/spot-advisor');
    expect(res.ok()).toBeTruthy();
  });

  test('finops reserved planner API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/reserved-planner');
    expect(res.ok()).toBeTruthy();
  });

  test('finops anomalies API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/anomalies');
    expect(res.ok()).toBeTruthy();
  });

  test('finops unit economics API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/unit-economics');
    expect(res.ok()).toBeTruthy();
  });

  test('finops multicloud compare API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/multicloud-compare');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.rows.length).toBe(3);
  });

  test('finops carbon API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/carbon');
    expect(res.ok()).toBeTruthy();
  });

  test('finops trends API', async ({ request }) => {
    const res = await request.get('/api/intelligence/finops/trends');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.points.length).toBeGreaterThan(0);
  });

  test('finops execute API', async ({ request }) => {
    const res = await request.post('/api/intelligence/finops/execute', {
      data: { dry_run: true },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('finops budget webhook API', async ({ request }) => {
    const res = await request.post('/api/intelligence/finops/budget-webhook', { data: {} });
    expect(res.ok()).toBeTruthy();
  });

  test('finops platform panel on cost page', async ({ page }) => {
    await page.goto('/cost');
    await expect(page.getByTestId('finops-platform-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('finops-trends-panel')).toBeVisible();
    await page.getByRole('button', { name: 'Multi-cloud' }).click();
    await expect(page.getByTestId('finops-multicloud-panel')).toBeVisible();
  });
});
