// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1831–1880 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1831: drift platform link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx' }],
        }),
      }),
    );
    await page.route('**/api/drift/web', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            workload_name: 'web',
            has_drift: false,
            severity: 'none',
            drifts: [],
            reconciliation_plan: [],
          },
        }),
      }),
    );
    await page.goto('/drift?workload=web');
    await expect(page.getByTestId('drift-workload-context')).toContainText('web', { timeout: 15_000 });
    await page.getByTestId('drift-workload-context').getByTestId('drift-links-more').click();
    await page.getByTestId('drift-workload-context').getByTestId('drift-platform-link').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1836: cost intelligence link', async ({ page }) => {
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 0,
            totalSpotMonthlyUsd: 0,
            tco36MonthsUsd: 0,
            pricingSource: 'aws',
            region: 'us-east-1',
            lines: [],
          },
        }),
      }),
    );
    await page.goto('/cost?workload=web');
    await expect(page.getByTestId('cost-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('cost-intelligence-link').click();
    await expect(page).toHaveURL(/\/intelligence\?workload=web.*tab=cost/, { timeout: 10_000 });
  });

  test('phase 1846: overview copilot link', async ({ page }) => {
    await page.goto('/?workload=web');
    await expect(page.getByTestId('overview-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('overview-copilot-link').click();
    await expect(page).toHaveURL(/\/copilot\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1851: backups secrets link', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/backups?workload=web');
    await expect(page.getByTestId('backups-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('backups-secrets-link').click();
    await expect(page).toHaveURL(/\/secrets\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1856: metrics drift link', async ({ page }) => {
    await page.route('**/api/metrics/observability**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { prometheus_up: true, grafana_up: false } }),
      }),
    );
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { totalMonthlyUsd: 0, totalSpotMonthlyUsd: 0, tco36MonthsUsd: 0, pricingSource: 'aws', region: 'us-east-1', lines: [] },
        }),
      }),
    );
    await page.goto('/metrics?workload=web');
    await expect(page.getByTestId('metrics-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('metrics-drift-link').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1861: scheduler affinity link', async ({ page }) => {
    await page.route('**/api/scheduler/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/scheduler?workload=web');
    await expect(page.getByTestId('scheduler-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('scheduler-affinity-link').click();
    await expect(page).toHaveURL(/\/affinity\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1866: workload detail compose quick link', async ({ page }) => {
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
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-compose').click();
    await expect(page).toHaveURL(/\/compose\?workload=web/, { timeout: 10_000 });
  });
});
