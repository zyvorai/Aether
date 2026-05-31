// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';
import { installDesignApiMocks, installLoginPageMocks, seedAuthenticatedSession, clearAuthenticatedSession } from './fixtures/designMocks';

const designRoutes = [
  { path: '/', expectTabs: false, expectBreadcrumb: false },
  { path: '/metrics', expectTabs: false, expectBreadcrumb: true },
  { path: '/workloads', expectTabs: false, expectBreadcrumb: true },
  { path: '/intelligence', expectTabs: true, expectBreadcrumb: true },
  { path: '/platform', expectTabs: false, expectBreadcrumb: true },
] as const;

test.describe('Secure design smoke', () => {
  test.beforeEach(async ({ page }) => {
    await installDesignApiMocks(page);
    await seedAuthenticatedSession(page);
  });

  for (const { path, expectTabs, expectBreadcrumb } of designRoutes) {
    test(`${path} renders secure shell tokens`, async ({ page }) => {
      await page.goto(path);
      await expect(page.locator('.overview-section-shell, .command-center-shell').first()).toBeVisible({
        timeout: 15_000,
      });

      if (expectBreadcrumb) {
        await expect(page.locator('nav.dash-breadcrumb')).toBeVisible({ timeout: 15_000 });
      }

      if (expectTabs) {
        await expect(page.locator('nav .tab-chip, .tab-chip').first()).toBeVisible();
      }
    });
  }

  test('navbar uses solid shell class', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#app-topnav')).toHaveClass(/navbar-solid/);
  });

  test('main section shell has no backdrop blur', async ({ page }) => {
    await page.goto('/');
    const blur = await page.locator('.overview-section-shell, .command-center-shell').first().evaluate((el) => {
      return window.getComputedStyle(el).backdropFilter;
    });
    expect(blur === 'none' || blur === '').toBeTruthy();
  });
});

test.describe('Secure design login', () => {
  test.beforeEach(async ({ page, context }) => {
    await context.clearCookies();
    await installLoginPageMocks(page);
    await clearAuthenticatedSession(page);
  });

  test('login form uses solid secure card', async ({ page }) => {
    await page.goto('/');
    const loginPage = page.locator('.login-page');
    const visible = await loginPage.isVisible().catch(() => false);
    if (!visible) {
      test.skip(true, 'Login gate hidden (dev bootstrap or cookie session active)');
    }
    await expect(page.locator('.login-glass')).toBeVisible({ timeout: 15_000 });
    const blur = await page.locator('.login-glass').evaluate((el) => {
      return window.getComputedStyle(el).backdropFilter;
    });
    expect(blur === 'none' || blur === '').toBeTruthy();
    await expect(page.getByText('RBAC-aware · Encrypted session')).toBeVisible();
  });
});
