// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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

test.describe('Phases 2231–2280 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 2231: orchestrator fleet link', async ({ page }) => {
    await page.goto('/health-monitor');
    await expect(page.getByTestId('health-hub-context')).toBeVisible({ timeout: 15_000 });
    await followTestLink(page, 'health-context-fleet-link', /\/fleet/);
  });

  test('phase 2236: intelligence orchestrator link', async ({ page }) => {
    await page.goto('/intelligence');
    await followTestLink(page, 'intelligence-context-orchestrator-link', /\/health-monitor/);
  });

  test('phase 2241: fleet intelligence link', async ({ page }) => {
    await page.goto('/fleet');
    await followTestLink(page, 'fleet-context-intelligence-link', /\/intelligence/);
  });

  test('phase 2246: settings rbac link', async ({ page }) => {
    await page.goto('/settings');
    await followTestLink(page, 'settings-context-rbac-link', /\/rbac/);
  });

  test('phase 2251: hosted billing panel', async ({ page }) => {
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
    await page.route('**/api/hosted/billing/usage', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            period: '2026-06',
            total_workloads: 0,
            stripe_configured: true,
            tenants: [],
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
    await page.route('**/api/hosted/federation', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { federation_enabled: false, tenant_count: 1, policy: { clusters: [], weights: {} } },
        }),
      }),
    );
    await page.goto('/hosted');
    await expect(page.getByTestId('hosted-billing-panel')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('hosted-stripe-checkout-button')).toBeVisible();
    await expect(page.getByTestId('hosted-stripe-portal-button')).toBeVisible();
  });
});
