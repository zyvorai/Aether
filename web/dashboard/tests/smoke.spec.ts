// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('Aether dashboard smoke', () => {
  test('health endpoint responds', async ({ request }) => {
    const res = await request.get('/health');
    expect(res.ok()).toBeTruthy();
  });

  test('dashboard HTML loads', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('body')).toBeVisible();
    const title = await page.title();
    expect(title.length).toBeGreaterThan(0);
  });

  test('login gate or dashboard shell visible', async ({ page }) => {
    await page.goto('/');
    const loginOrNav = page.getByRole('button', { name: /sign in|dashboard|workloads/i }).first();
    await expect(loginOrNav).toBeVisible({ timeout: 15_000 });
  });
});
