// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1231–1280 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1231: audit workload context links to trust', async ({ page }) => {
    const auditPayload = {
      success: true,
      data: {
        summary: {
          total_events: 1,
          successes: 1,
          failures: 0,
          unique_workloads: 1,
          events_by_action: { deploy: 1 },
        },
        recent_events: [
          {
            id: 1,
            timestamp: new Date().toISOString(),
            action: 'deploy',
            workload: 'web',
            result: 'success',
            runtime: 'kube',
            message: 'ok',
          },
        ],
      },
    };
    await page.route('**/api/audit/verify', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total: 1, verified: 1, tampered: 0, integrity: 'ok', tampered_events: [] },
        }),
      }),
    );
    await page.route('**/api/audit', (route) => {
      if (route.request().url().includes('/verify')) return route.fallback();
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(auditPayload) });
    });
    await page.goto('/audit?workload=web');
    await expect(page.getByTestId('audit-trust-link')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('audit-trust-link').first().click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1232: drift workload context links to trust', async ({ page }) => {
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
    await page.route('**/api/drift/web', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { has_drift: false, fields: [] } }),
      }),
    );
    await page.goto('/drift?workload=web');
    await expect(page.getByTestId('drift-workload-context')).toContainText('web');
    const trustHref = await page.getByTestId('drift-trust-link').getAttribute('href');
    expect(trustHref).toMatch(/tab=trust/);
    await page.goto(trustHref!);
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1236: copilot workload context and trust link', async ({ page }) => {
    await page.goto('/copilot?workload=web');
    await expect(page.getByTestId('copilot-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('copilot-trust-link').click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1238: events workload filter links to trust tab', async ({ page }) => {
    await page.route('**/api/events/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total_events: 1, unacknowledged: 0, critical_unacked: 0 },
        }),
      }),
    );
    await page.route('**/api/events**', (route) => {
      if (route.request().url().includes('/summary')) return route.continue();
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      });
    });
    await page.goto('/events?workload=web');
    await page.getByTestId('events-trust-link').first().click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1238b: overview trust quick link opens trust tab', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-trust-quick-link').click();
    await expect(page).toHaveURL(/tab=trust/, { timeout: 10_000 });
  });

  test('phase 1235: secrets search context links to trust', async ({ page }) => {
    await page.route('**/api/secrets', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/secrets?q=vm-web');
    await page.getByTestId('secrets-trust-link').click();
    await expect(page).toHaveURL(/workload=vm-web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1234: scheduler workload context links to alerts', async ({ page }) => {
    await page.route('**/api/scheduler/utilization', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/scheduler/suggestions', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/scheduler?workload=web');
    await expect(page.getByTestId('scheduler-workload-context')).toContainText('web');
    await page.getByTestId('scheduler-alerts-link').click();
    await expect(page).toHaveURL(/\/alerts\?workload=web/, { timeout: 10_000 });
  });
});
