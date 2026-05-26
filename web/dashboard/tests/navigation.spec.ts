// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('Dashboard navigation', () => {
  test('API version endpoint', async ({ request }) => {
    const res = await request.get('/api/dashboard/version');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data?.api_version ?? body.api_version).toBeTruthy();
  });

  test('platform page route loads', async ({ page }) => {
    await page.goto('/platform');
    await expect(page.getByRole('heading', { name: /runtime/i }).first()).toBeVisible({ timeout: 15_000 });
  });

  test('events page with category filter', async ({ page }) => {
    await page.goto('/events?category=workload');
    await expect(page.locator('body')).toBeVisible();
    await expect(page.getByText(/events/i).first()).toBeVisible({ timeout: 15_000 });
  });
});
