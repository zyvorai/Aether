// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect, type Page } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

async function followTestLink(page: Page, testId: string, urlPattern: RegExp) {
  const link = page.getByTestId(testId);
  await expect(link).toBeVisible({ timeout: 10_000 });
  const href = await link.getAttribute('href');
  expect(href).toMatch(urlPattern);
  await page.goto(href!);
  await expect(page).toHaveURL(urlPattern, { timeout: 10_000 });
}

test.describe('Phases 2181–2230 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 2181: settings fleet link', async ({ page }) => {
    await page.goto('/settings');
    await followTestLink(page, 'settings-context-fleet-link', /\/fleet/);
  });

  test('phase 2191: backups settings link', async ({ page }) => {
    await page.goto('/backups');
    await followTestLink(page, 'backups-context-settings-link', /\/settings/);
  });

  test('phase 2196: hosted federation panel', async ({ page }) => {
    await page.route('**/api/hosted/tenants', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ id: 't1', name: 'Acme', slug: 'acme', plan: 'team', active: true }],
        }),
      }),
    );
    await page.route('**/api/hosted/federation', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            federation_enabled: true,
            tenant_count: 1,
            policy: { clusters: ['east', 'west'], weights: { east: 1, west: 0.8 } },
          },
        }),
      }),
    );
    await page.route('**/api/hosted/upgrades', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            current_version: '1.0.0',
            channel: 'stable',
            managed_upgrades: true,
            upgrade_available: false,
            notes: 'test',
          },
        }),
      }),
    );
    await page.route('**/api/hosted/billing/usage', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { period: '2026-06', total_workloads: 0, tenants: [] },
        }),
      }),
    );
    await page.goto('/hosted');
    await expect(page.getByTestId('hosted-federation-panel')).toBeVisible({ timeout: 10_000 });
    await followTestLink(page, 'hosted-context-federation-link', /\/fleet/);
  });

  test('phase 2186: gitops hosted link', async ({ page }) => {
    await page.goto('/gitops?workload=web');
    await followTestLink(page, 'gitops-context-hosted-link', /\/hosted/);
  });
});
