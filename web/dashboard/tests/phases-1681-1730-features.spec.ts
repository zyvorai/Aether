// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1681–1730 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1681: cost workload context metrics link', async ({ page }) => {
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 10,
            totalSpotMonthlyUsd: 0,
            tco36MonthsUsd: 0,
            pricingSource: 'aws',
            region: 'us-east-1',
            lines: [{ workload: 'web', owner: '', project: '', monthlyUsd: 10 }],
          },
        }),
      }),
    );
    await page.goto('/cost?workload=web');
    await expect(page.getByTestId('cost-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('cost-workload-context').getByTestId('cost-metrics-link').click();
    await expect(page).toHaveURL(/\/metrics\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1686: scheduler workload context cost link', async ({ page }) => {
    await page.route('**/api/scheduler/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/scheduler?workload=web');
    await expect(page.getByTestId('scheduler-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('scheduler-context-cost-link').click();
    await expect(page).toHaveURL(/\/cost\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1691: plugins editor link from workload filter', async ({ page }) => {
    await page.route('**/api/plugins', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/plugins/discover', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { discovered: 0 } }) }),
    );
    await page.goto('/plugins?workload=web');
    await expect(page.getByTestId('plugins-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('plugins-editor-link').click();
    await expect(page).toHaveURL(/\/editor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1696: platform clusters cross-link', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { version: '0.3.0' } }) }),
    );
    await page.route('**/api/platform/recommendations', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { items: [] } }) }),
    );
    await page.goto('/platform?workload=web');
    await expect(page.getByTestId('platform-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('platform-links-more').click();
    await page.getByTestId('platform-clusters-link').click();
    await expect(page).toHaveURL(/\/clusters\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1698: clusters platform cross-link', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { enabled: true, connected: true, cluster_count: 1, healthy_clusters: 1, clusters: [{ name: 'local', reachable: true }] },
        }),
      }),
    );
    await page.route('**/api/cluster/namespaces**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: ['default'] }) }),
    );
    await page.route('**/api/cluster/browse**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/cluster/metrics**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.route('**/api/server', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: {} }) }),
    );
    await page.goto('/clusters?workload=web');
    await expect(page.getByTestId('clusters-workload-context')).toContainText('web', { timeout: 15_000 });
    await page.getByTestId('clusters-platform-link').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1700: overview platform link', async ({ page }) => {
    await page.goto('/?workload=web');
    await expect(page.getByTestId('overview-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('overview-platform-link').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1688: ai intelligence link', async ({ page }) => {
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
    await expect(page.getByTestId('ai-workload-context')).toContainText('web', { timeout: 15_000 });
    await page.getByTestId('ai-context-intelligence-link').click();
    await expect(page).toHaveURL(/\/intelligence\?workload=web/, { timeout: 10_000 });
  });
});
