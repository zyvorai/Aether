// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from '@playwright/test';
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
  '/copilot',
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
] as const;

const GLASS_SHELL = '.command-center-shell, .overview-section-shell';

async function assertGlassBlur(page: import('@playwright/test').Page, shell: string) {
  const glassShell = page.locator(shell).first();
  await expect(glassShell).toBeVisible({ timeout: 15_000 });
  const backdropFilter = await glassShell.evaluate((el) => getComputedStyle(el).backdropFilter);
  expect(backdropFilter).not.toBe('none');
}

async function selectTheme(page: import('@playwright/test').Page, theme: 'dark' | 'steel' | 'aurora') {
  const themeSelect = page.locator('nav select').first();
  await expect(themeSelect).toBeVisible({ timeout: 10_000 });
  await themeSelect.selectOption(theme === 'dark' ? 'dark' : theme);
  if (theme === 'dark') {
    await expect(page.locator('html')).not.toHaveClass(/steel-theme|aurora-theme/);
  } else {
    await expect(page.locator('html')).toHaveClass(new RegExp(`${theme}-theme`));
  }
}

test.describe('Liquid Glass smoke', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  for (const path of ALL_DASHBOARD_PATHS) {
    test(`glass shell visible with blur on ${path}`, async ({ page }) => {
      await page.goto(path);
      await assertGlassBlur(page, GLASS_SHELL);
    });
  }

  for (const theme of ['steel', 'aurora'] as const) {
    test(`all dashboard routes retain glass blur with ${theme} theme`, async ({ page }) => {
      await page.goto('/workloads');
      await selectTheme(page, theme);

      for (const path of ALL_DASHBOARD_PATHS) {
        await page.goto(path);
        await assertGlassBlur(page, GLASS_SHELL);
      }
    });
  }

  test('clusters detail uses glass-drawer when resource selected', async ({ page }) => {
    await page.goto('/clusters');
    const empty = page.getByText('No Kubernetes contexts').first();
    const table = page.getByTestId('clusters-resource-table');
    const hasTable = await table.isVisible({ timeout: 15_000 }).catch(() => false);
    if (!hasTable) {
      await expect(empty.or(page.getByRole('heading', { name: /cluster browser/i }))).toBeVisible();
      return;
    }

    const firstRow = table.locator('tbody tr').first();
    await firstRow.click();
    await expect(page.locator('.glass-drawer').first()).toBeVisible({ timeout: 10_000 });
  });

  test('workloads detail panel uses glass tabs when workload selected', async ({ page }) => {
    await page.goto('/workloads');
    const table = page.locator('table tbody tr');
    const hasRows = await table.first().isVisible({ timeout: 15_000 }).catch(() => false);
    if (!hasRows) {
      await expect(page.locator('.overview-section-shell').first()).toBeVisible();
      return;
    }
    await table.first().click();
    const detail = page.locator('.glass-panel-card .glass-tab, .glass-panel-card .tab-chip').first();
    await expect(detail).toBeVisible({ timeout: 10_000 });
    const panel = page.locator('.glass-panel-card').filter({ has: detail }).first();
    const backdropFilter = await panel.evaluate((el) => getComputedStyle(el).backdropFilter);
    expect(backdropFilter).not.toBe('none');
  });

  test('validate modal uses glass-modal-panel', async ({ page }) => {
    await page.goto('/workloads?validate=1');
    const panel = page.locator('.glass-modal-panel').first();
    await expect(panel).toBeVisible({ timeout: 15_000 });
    const backdropFilter = await panel.evaluate((el) => getComputedStyle(el).backdropFilter);
    expect(backdropFilter).not.toBe('none');
  });

  test('copilot page uses glass inset surfaces', async ({ page }) => {
    await page.goto('/copilot');
    await expect(page.locator('.overview-section-shell').first()).toBeVisible({ timeout: 15_000 });
    const inset = page.locator('.glass-inset-surface, .glass-panel-card').first();
    await expect(inset).toBeVisible({ timeout: 10_000 });
  });

  test('fleet page tab chips use glass-tab', async ({ page }) => {
    await page.goto('/fleet');
    await expect(page.locator('.overview-section-shell').first()).toBeVisible({ timeout: 15_000 });
    const tab = page.locator('.glass-tab').first();
    await expect(tab).toBeVisible({ timeout: 10_000 });
  });
});
