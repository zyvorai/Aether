// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Apple.com chapter alignment', () => {
  test('login gate uses paper store chapters', async ({ page }) => {
    await page.goto('/');
    const login = page.getByTestId('login-gate');
    const nav = page.getByTestId('global-nav');
    if (await login.isVisible().catch(() => false)) {
      await expect(login.locator('.login-hero')).toBeVisible();
      await expect(login.getByTestId('local-login-form')).toBeVisible();
      await expect(login.locator('.login-btn-primary')).toBeVisible();
      return;
    }
    await expect(nav).toBeVisible({ timeout: 15_000 });
  });

  test('overview shows ink highlights when authenticated', async ({ page }) => {
    await ensureAuthenticated(page);
    await page.goto('/');
    await expect(page.getByTestId('global-nav')).toBeVisible({ timeout: 15_000 });
    const highlights = page.getByTestId('overview-apple-highlights');
    const empty = page.getByText(/Deploy your first workload/i);
    // Either fleet highlights or empty-platform onboarding
    await expect(highlights.or(empty).first()).toBeVisible({ timeout: 20_000 });
  });

  test('Aurora PageHero uses display title scale', async ({ page }) => {
    await ensureAuthenticated(page);
    await page.goto('/workloads');
    await expect(page.getByTestId('page-workloads')).toBeVisible({ timeout: 15_000 });
    await expect(page.locator('.apple-editorial-hero').first()).toBeVisible();
  });
});
