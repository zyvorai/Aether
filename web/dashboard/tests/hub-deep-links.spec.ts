// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Links that carry filters', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('a filtered hub address opens the sub-page and keeps the whole query', async ({ page }) => {
    await page.goto('/platform?workload=web&ref=mail');
    await expect(page).toHaveURL(/\/platform\/runtime\?workload=web&ref=mail$/);
    await expect(page.getByTestId('platform-workload-context')).toContainText('web');
  });

  test('Back leaves the redirected page instead of bouncing', async ({ page }) => {
    await page.goto('/events');
    await page.goto('/clusters?kind=Node');
    await expect(page).toHaveURL(/\/clusters\/browse\?kind=Node$/);
    await page.goBack();
    await expect(page).toHaveURL(/\/events$/);
    await page.goForward();
    await expect(page).toHaveURL(/\/clusters\/browse\?kind=Node$/);
  });

  test('the command palette reaches the sub-page', async ({ page }) => {
    await page.goto('/events');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('intent infrastructure pipeline');
    await page.getByRole('button', { name: 'Run intent to infrastructure pipeline' }).click();
    await expect(page).toHaveURL(/\/ai\/pipeline\?tab=pipeline$/);
    await page.goBack();
    await expect(page).toHaveURL(/\/events$/);
  });

  test('a hub without filters stays a hub', async ({ page }) => {
    await page.goto('/clusters');
    await expect(page.getByTestId('page-clusters')).toBeVisible();
    await expect(page).toHaveURL(/\/clusters$/);
  });

  test('Summary shows the hub, which keeps the workload', async ({ page }) => {
    await page.goto('/platform?workload=web');
    const tabs = page.getByTestId('section-tabs');
    await tabs.getByRole('link', { name: 'Summary' }).click();
    await expect(page.getByTestId('page-platform')).toBeVisible();
    await expect(page).toHaveURL(/\/platform\?workload=web$/);
    await page.goBack();
    await expect(page).toHaveURL(/\/platform\/runtime\?workload=web$/);
    await page.goForward();
    await expect(page).toHaveURL(/\/platform\?workload=web$/);
    await page.locator('a.hub-band[href^="/platform/trust"]').click();
    await expect(page).toHaveURL(/\/platform\/trust\?workload=web$/);
  });
});
