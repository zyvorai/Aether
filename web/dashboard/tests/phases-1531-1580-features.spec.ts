// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1531–1580 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1531: palette opens SLA for workload', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx', source: 'cluster' }],
        }),
      }),
    );
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await expect(page.getByTestId('command-palette')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('command-palette-input').fill('sla web');
    await page.getByRole('button', { name: /SLA compliance: web/i }).click();
    await expect(page).toHaveURL(/\/sla\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1537: palette opens policy for workload', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx', source: 'cluster' }],
        }),
      }),
    );
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await expect(page.getByTestId('command-palette-input')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('command-palette-input').fill('policy web');
    await page.getByRole('button', { name: /Policy check: web/i }).click();
    await expect(page).toHaveURL(/\/policy\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1560: command palette has testids', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await expect(page.getByTestId('command-palette-input')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('command-palette-input').fill('zzznomatchxyz');
    await expect(page.getByTestId('command-palette-empty')).toBeVisible({ timeout: 5_000 });
  });
});
