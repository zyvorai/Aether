// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 51–100 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 51–52: cost page links to intelligence cost tab', async ({ page }) => {
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 10,
            totalSpotMonthlyUsd: 5,
            tco36MonthsUsd: 360,
            pricingSource: 'aws',
            region: 'us-east-1',
            lines: [],
          },
        }),
      }),
    );
    await page.goto('/cost');
    await page.getByRole('link', { name: 'Cost optimize →' }).click();
    await expect(page).toHaveURL(/\/intelligence.*tab=cost|tab=cost.*\/intelligence/, { timeout: 10_000 });
  });

  test('phase 53: metrics chargeback workload link', async ({ page }) => {
    await page.route('**/api/metrics', (route) => route.fulfill({ status: 200, body: '# HELP test\n' }));
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { integrations: {} } }),
      }),
    );
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 42,
            totalSpotMonthlyUsd: 10,
            tco36MonthsUsd: 1500,
            pricingSource: 'aws',
            region: 'us-east-1',
            lines: [{ workload: 'web', owner: 'team-a', project: 'demo', monthlyUsd: 42 }],
          },
        }),
      }),
    );
    await page.route('**/api/observability/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { prometheus_configured: false, api_http_requests_total: 0, migrations_total: 0, migration_rollbacks_total: 0, workloads_running: {} },
        }),
      }),
    );

    await page.goto('/metrics');
    await page.getByRole('button', { name: 'web' }).click();
    await expect(page).toHaveURL(/workload=web/, { timeout: 10_000 });
  });

  test('phase 54: openapi download JSON button', async ({ page }) => {
    await page.route('**/api/openapi.json', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          openapi: '3.0.0',
          info: { title: 'Aether', version: '0.3.0' },
          paths: { '/api/workloads': { get: { summary: 'List' } } },
        }),
      }),
    );
    await page.goto('/openapi');
    await expect(page.getByTestId('openapi-download-json')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 55: command palette opens fleet', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('fleet');
    await page.getByRole('button', { name: 'Open fleet overview' }).click();
    await expect(page).toHaveURL(/\/fleet/, { timeout: 10_000 });
  });

  test('phase 56: envs promote result panel', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            { name: 'dev', tier: 'development', workloads: {}, variables: {}, updated_at: new Date().toISOString() },
            { name: 'prod', tier: 'production', workloads: {}, variables: {}, updated_at: new Date().toISOString() },
          ],
        }),
      }),
    );
    await page.route('**/api/environments/promote', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { promoted: true } }),
      }),
    );
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { authenticated: true, role: 'operator', username: 'admin' } }),
      }),
    );

    await page.goto('/envs');
    await expect(page.getByText('Promote workload')).toBeVisible({ timeout: 10_000 });
    await page.locator('input[placeholder="Workload name"]').fill('web');
    await page.locator('input[placeholder="From env"]').fill('dev');
    await page.locator('input[placeholder="To env"]').fill('prod');
    await page.getByTestId('envs-promote-submit').click();
    await expect(page.getByTestId('envs-promote-result')).toContainText('dev → prod', { timeout: 10_000 });
  });

  test('phase 57: overview scheduler quick link', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Scheduler' }).click();
    await expect(page).toHaveURL(/\/scheduler/, { timeout: 10_000 });
  });

  test('phase 58: templates configure modal', async ({ page }) => {
    await page.route('**/api/templates', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web-service', description: 'Web app', default_cpu: '500m', default_memory: '512Mi' }],
        }),
      }),
    );
    await page.goto('/templates');
    await page.getByRole('button', { name: 'Configure' }).click();
    await expect(page.getByTestId('template-configure-modal')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 59: alerts webhook queue panel', async ({ page }) => {
    await page.route('**/api/alerts/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { channels: [], rules: [] } }),
      }),
    );
    await page.route('**/api/webhooks/queue', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/alerts');
    await expect(page.getByTestId('alerts-webhook-queue')).toBeVisible({ timeout: 10_000 });
  });
});
