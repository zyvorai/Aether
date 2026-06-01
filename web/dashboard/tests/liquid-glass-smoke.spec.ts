// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

const GLASS_ROUTES = [
  { path: '/', shell: '.command-center-shell, .overview-section-shell' },
  { path: '/clusters', shell: '.overview-section-shell' },
  { path: '/workloads', shell: '.overview-section-shell' },
  { path: '/editor', shell: '.overview-section-shell' },
  { path: '/helm', shell: '.overview-section-shell' },
  { path: '/secrets', shell: '.overview-section-shell' },
  { path: '/settings', shell: '.overview-section-shell' },
];

async function assertGlassBlur(page: import('@playwright/test').Page, shell: string) {
  const glassShell = page.locator(shell).first();
  await expect(glassShell).toBeVisible({ timeout: 15_000 });
  const backdropFilter = await glassShell.evaluate((el) => getComputedStyle(el).backdropFilter);
  expect(backdropFilter).not.toBe('none');
}

test.describe('Liquid Glass smoke', () => {
  for (const { path, shell } of GLASS_ROUTES) {
    test(`glass shell visible with blur on ${path}`, async ({ page }) => {
      await page.goto(path);
      await assertGlassBlur(page, shell);
    });
  }

  for (const theme of ['steel', 'aurora'] as const) {
    test(`overview shell blur with ${theme} theme`, async ({ page }) => {
      await page.goto('/workloads');
      await assertGlassBlur(page, '.overview-section-shell');

      const themeSelect = page.locator('nav select').first();
      await expect(themeSelect).toBeVisible({ timeout: 10_000 });
      await themeSelect.selectOption(theme);

      await expect(page.locator('html')).toHaveClass(new RegExp(`${theme}-theme`));
      await assertGlassBlur(page, '.overview-section-shell');
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
    await ensureAuthenticated(page);
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
    await ensureAuthenticated(page);
    await page.goto('/workloads?validate=1');
    const panel = page.locator('.glass-modal-panel').first();
    await expect(panel).toBeVisible({ timeout: 15_000 });
    const backdropFilter = await panel.evaluate((el) => getComputedStyle(el).backdropFilter);
    expect(backdropFilter).not.toBe('none');
  });

  test('copilot page uses glass inset surfaces', async ({ page }) => {
    await ensureAuthenticated(page);
    await page.goto('/copilot');
    await expect(page.locator('.overview-section-shell').first()).toBeVisible({ timeout: 15_000 });
    const inset = page.locator('.glass-inset-surface, .glass-panel-card').first();
    await expect(inset).toBeVisible({ timeout: 10_000 });
  });

  test('fleet page tab chips use glass-tab', async ({ page }) => {
    await ensureAuthenticated(page);
    await page.goto('/fleet');
    await expect(page.locator('.overview-section-shell').first()).toBeVisible({ timeout: 15_000 });
    const tab = page.locator('.glass-tab').first();
    await expect(tab).toBeVisible({ timeout: 10_000 });
  });
});
