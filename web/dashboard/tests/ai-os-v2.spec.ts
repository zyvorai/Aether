// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v2', () => {
  test('digital twin simulate API', async ({ request }) => {
    const res = await request.post('/api/intelligence/digital-twin/simulate', {
      data: { scale_factor: 1.5 },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.scenario).toBeTruthy();
    expect(data.baseline).toBeTruthy();
    expect(data.projected).toBeTruthy();
  });

  test('security copilot policies API', async ({ request }) => {
    const res = await request.get('/api/intelligence/security/policies');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.suggestions)).toBeTruthy();
  });

  test('capacity forecast panel on observability', async ({ page }) => {
    await page.goto('/observability');
    await expect(page.getByTestId('capacity-forecast-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('cost intelligence panel on cost page', async ({ page }) => {
    await page.goto('/cost');
    await expect(page.getByTestId('cost-intelligence-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('security copilot panel on security page', async ({ page }) => {
    await page.goto('/security');
    await expect(page.getByTestId('security-copilot-panel')).toBeVisible({ timeout: 15_000 });
  });

  test('digital twin panel on fabric page', async ({ page }) => {
    await page.goto('/fabric');
    await expect(page.getByTestId('digital-twin-panel')).toBeVisible({ timeout: 15_000 });
  });
});
