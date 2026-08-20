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

test.describe('Phases 2131–2180 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 2131: fabric fleet link', async ({ page }) => {
    await page.goto('/fabric?workload=web');
    await expect(page.getByTestId('fabric-workload-context')).toContainText('web', { timeout: 10_000 });
    await followTestLink(page, 'fabric-context-fleet-link', /\/fleet\?workload=web/);
  });

  test('phase 2136: migrations fabric link', async ({ page }) => {
    await page.goto('/migrations?workload=web');
    await expect(page.getByTestId('migrations-workload-context')).toContainText('web', { timeout: 10_000 });
    await followTestLink(page, 'mig-context-fabric-link', /\/fabric\?workload=web/);
  });

  test('phase 2141: settings platform link', async ({ page }) => {
    await page.goto('/settings');
    await followTestLink(page, 'settings-context-platform-link', /\/platform/);
  });

  test('phase 2146: security rbac link', async ({ page }) => {
    await page.goto('/security?workload=web');
    await expect(page.getByTestId('security-workload-context')).toContainText('web', { timeout: 10_000 });
    await followTestLink(page, 'security-context-rbac-link', /\/rbac\?workload=web/);
  });

  test('phase 2151: zyra fabric link', async ({ page }) => {
    await page.goto('/copilot?workload=web');
    await expect(page.getByTestId('zyra-workload-context')).toContainText('web', { timeout: 10_000 });
    await followTestLink(page, 'zyra-context-fabric-link', /\/fabric\?workload=web/);
  });

  test('phase 2156: hosted platform link', async ({ page }) => {
    await page.route('**/api/hosted/tenants', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ id: 't1', name: 'Acme', slug: 'acme', plan: 'free', active: true }],
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
    await followTestLink(page, 'hosted-context-platform-link', /\/platform/);
  });
});
