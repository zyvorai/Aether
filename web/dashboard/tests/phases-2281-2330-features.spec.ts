// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect, type Page } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';
import { testId } from './helpers/locators';

async function followTestLink(page: Page, id: string, urlPattern: RegExp) {
  const link = testId(page, id);
  await expect(link).toBeVisible({ timeout: 10_000 });
  const href = await link.getAttribute('href');
  expect(href).toMatch(urlPattern);
  await page.goto(href!);
  await expect(page).toHaveURL(urlPattern, { timeout: 10_000 });
}

test.describe('Phases 2281–2330 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 2281: migrations orchestrator link', async ({ page }) => {
    await page.goto('/migrations');
    await expect(page.getByTestId('migrations-hub-context')).toBeVisible({ timeout: 15_000 });
    await followTestLink(page, 'migrations-context-orchestrator-link', /\/health-monitor/);
  });

  test('phase 2286: security intelligence link', async ({ page }) => {
    await page.goto('/security');
    await expect(page.getByTestId('security-hub-context')).toBeVisible({ timeout: 15_000 });
    await followTestLink(page, 'security-context-intelligence-link', /\/intelligence/);
  });

  test('phase 2291: cost hosted link', async ({ page }) => {
    await page.goto('/cost');
    await expect(page.getByTestId('cost-hub-context')).toBeVisible({ timeout: 15_000 });
    await followTestLink(page, 'cost-context-hosted-link', /\/hosted/);
  });

  test('phase 2296: platform fleet link', async ({ page }) => {
    await page.goto('/platform');
    await expect(page.getByTestId('platform-hub-context')).toBeVisible({ timeout: 15_000 });
    await followTestLink(page, 'platform-context-fleet-hub-link', /\/fleet/);
  });

  test('phase 2301: backups migrations link', async ({ page }) => {
    await page.goto('/backups');
    await expect(page.getByTestId('backups-hub-context')).toBeVisible({ timeout: 15_000 });
    await followTestLink(page, 'backups-context-migrations-link', /\/migrations/);
  });
});
