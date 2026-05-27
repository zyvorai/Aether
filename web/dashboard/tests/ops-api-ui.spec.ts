// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('Ops API dashboard wiring', () => {
  test('environments page shows promote and parity forms', async ({ page }) => {
    await page.goto('/envs');
    await expect(page.getByText(/promote workload/i).first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/environment parity/i).first()).toBeVisible();
  });

  test('sla page shows add target form', async ({ page }) => {
    await page.goto('/sla');
    await expect(page.getByText(/add sla target/i).first()).toBeVisible({ timeout: 15_000 });
  });

  test('metrics page shows prometheus query when configured', async ({ page }) => {
    await page.goto('/metrics');
    await expect(page.getByText(/prometheus metrics/i).first()).toBeVisible({ timeout: 15_000 });
  });

  test('gitops page loads init or sync controls', async ({ page }) => {
    await page.goto('/gitops');
    await expect(page.getByText(/gitops reconciliation/i).first()).toBeVisible({ timeout: 15_000 });
  });

  test('alerts page shows webhook queue section', async ({ page }) => {
    await page.goto('/alerts');
    await expect(page.getByText(/webhook retry queue/i).first()).toBeVisible({ timeout: 15_000 });
  });
});
