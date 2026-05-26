// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('Confidential & intelligence UI', () => {
  test('confidential hub loads', async ({ page }) => {
    await page.goto('/confidential');
    await expect(page.getByText(/confidential workloads/i).first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/encrypted migration/i).first()).toBeVisible();
  });

  test('intelligence layer page loads', async ({ page }) => {
    await page.goto('/intelligence');
    await expect(page.getByText(/predictions|fleet risk/i).first()).toBeVisible({ timeout: 15_000 });
  });

  test('fleet overview page loads', async ({ page }) => {
    await page.goto('/fleet');
    await expect(page.getByText(/registered clusters|network observability/i).first()).toBeVisible({
      timeout: 15_000,
    });
  });
});
