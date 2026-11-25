// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

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

  test('theme menu switches and persists light and dark', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('global-nav')).toBeVisible({ timeout: 15_000 });
    const html = page.locator('html');
    const toggle = page.getByTestId('theme-toggle');

    await toggle.click();
    await page.getByRole('menuitemradio', { name: 'Dark' }).click();
    await expect(html).toHaveClass(/dark-theme/);
    await expect(html).toHaveAttribute('data-theme', 'dark');
    await page.reload();
    await expect(html).toHaveAttribute('data-theme', 'dark');

    await toggle.click();
    await page.getByRole('menuitemradio', { name: 'Light' }).click();
    await expect(html).toHaveClass(/light-theme/);
    await expect(html).toHaveAttribute('data-theme', 'light');
  });

  test('system theme follows prefers-color-scheme', async ({ page }) => {
    await page.emulateMedia({ colorScheme: 'dark' });
    await page.goto('/');
    await page.getByTestId('theme-toggle').click();
    await page.getByRole('menuitemradio', { name: 'System' }).click();
    const html = page.locator('html');
    await expect(html).toHaveAttribute('data-theme', 'dark');
    await page.emulateMedia({ colorScheme: 'light' });
    await expect(html).toHaveAttribute('data-theme', 'light');
  });

  test('theme menu works from the keyboard', async ({ page }) => {
    await page.goto('/');
    const toggle = page.getByTestId('theme-toggle');
    await toggle.focus();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('menu', { name: 'Theme' })).toBeVisible();
    await page.keyboard.press('Home');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await expect(toggle).toBeFocused();
  });
});
