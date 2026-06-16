// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1931–1980 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1931: rbac policy link', async ({ page }) => {
    await page.goto('/rbac?workload=web');
    await expect(page.getByTestId('rbac-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('rbac-context-policy-link').click();
    await expect(page).toHaveURL(/\/policy\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1936: policy rbac link', async ({ page }) => {
    await page.goto('/policy?workload=web');
    await expect(page.getByTestId('policy-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('policy-context-rbac-link').click();
    await expect(page).toHaveURL(/\/rbac\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1941: ai copilot link', async ({ page }) => {
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
    await page.goto('/ai?workload=web');
    await expect(page.getByTestId('ai-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('ai-context-copilot-link').click();
    await expect(page).toHaveURL(/\/copilot\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1946: fleet policy link', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { enabled: true, connected: true, cluster_count: 1, healthy_clusters: 1, workload_count: 1, clusters: [] },
        }),
      }),
    );
    await page.route('**/api/server', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: {} }) }),
    );
    await page.goto('/fleet?workload=web');
    await expect(page.getByTestId('fleet-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('fleet-context-policy-link').click();
    await expect(page).toHaveURL(/\/policy\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1951: metrics intelligence link', async ({ page }) => {
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
    await page.getByTestId('metrics-context-intelligence-link').click();
    await expect(page).toHaveURL(/\/intelligence\?workload=web.*tab=predictions/, { timeout: 10_000 });
  });

  test('phase 1961: compose secrets link', async ({ page }) => {
    await page.goto('/compose?workload=web');
    await expect(page.getByTestId('compose-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('compose-secrets-link').click();
    await expect(page).toHaveURL(/\/secrets\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1971: workload detail plugins quick link', async ({ page }) => {
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
    await page.getByTestId('workload-link-plugins').click();
    await expect(page).toHaveURL(/\/plugins\?q=web/, { timeout: 10_000 });
  });
});
