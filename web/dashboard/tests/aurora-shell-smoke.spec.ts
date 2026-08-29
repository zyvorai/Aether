// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

/** All dashboard routes — keep in sync with DASHBOARD_VIEWS in src/utils/dashboardNav.ts */
const ALL_DASHBOARD_PATHS = [
  '/',
  '/fabric',
  '/migrations',
  '/observability',
  '/labs',
  '/settings',
  '/applications',
  '/workloads',
  '/ai',
  '/zyra',
  '/cost',
  '/affinity',
  '/drift',
  '/intelligence',
  '/policy',
  '/confidential',
  '/clusters',
  '/fleet',
  '/hosted',
  '/activity',
  '/helm',
  '/security',
  '/compose',
  '/editor',
  '/scheduler',
  '/health-monitor',
  '/events',
  '/alerts',
  '/platform',
  '/sla',
  '/deps',
  '/envs',
  '/secrets',
  '/backups',
  '/templates',
  '/plugins',
  '/rbac',
  '/audit',
  '/gitops',
  '/metrics',
  '/openapi',
  '/settings/ai-providers',
] as const;

test.describe('Aurora shell smoke', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('GlobalNav is present after login', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('global-nav')).toBeVisible({ timeout: 15_000 });
  });

  test('login demo hint or authenticated shell', async ({ page }) => {
    await page.goto('/');
    const nav = page.getByTestId('global-nav');
    const login = page.getByTestId('login-gate');
    const hasNav = await nav.isVisible().catch(() => false);
    const hasLogin = await login.isVisible().catch(() => false);
    expect(hasNav || hasLogin).toBeTruthy();
  });

  for (const path of ALL_DASHBOARD_PATHS) {
    test(`route ${path} shows GlobalNav + page content`, async ({ page }) => {
      await page.goto(path);
      await expect(page.getByTestId('global-nav')).toBeVisible({ timeout: 15_000 });
      // Aurora PageHero title or page test id
      const main = page.locator('#main-content, main, [data-testid^="page-"]').first();
      await expect(main).toBeVisible({ timeout: 15_000 });
    });
  }

  test('theme toggle switches dark-theme class', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('global-nav')).toBeVisible({ timeout: 15_000 });
    const themeBtn = page.getByTestId('global-nav').getByRole('button', { name: /switch to (light|dark)/i });
    if (await themeBtn.isVisible().catch(() => false)) {
      const before = await page.locator('html').evaluate((el) => el.classList.contains('dark-theme'));
      await themeBtn.click();
      const after = await page.locator('html').evaluate((el) => el.classList.contains('dark-theme'));
      expect(after).not.toBe(before);
    }
  });
});
