// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

function mockPlatform(page: import('@playwright/test').Page) {
  void page.route('**/api/server', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true, data: { version: '0.3.0', integrations: {} } }),
    }),
  );
  void page.route('**/api/platform/recommendations', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { items: [] } }) }),
  );
}

function mockClusters(page: import('@playwright/test').Page) {
  void page.route('**/api/cluster/summary', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        data: { enabled: true, connected: true, cluster_count: 1, healthy_clusters: 1, clusters: [{ name: 'local', reachable: true }] },
      }),
    }),
  );
  void page.route('**/api/cluster/namespaces**', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: ['default'] }) }),
  );
  void page.route('**/api/cluster/browse**', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        data: [{ name: 'web', kind: 'Deployment', namespace: 'default', status: 'Running', manifest: {}, created_at: new Date().toISOString() }],
      }),
    }),
  );
  void page.route('**/api/cluster/metrics**', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
  );
  void page.route('**/api/server', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: {} }) }),
  );
}

function mockOpenApi(page: import('@playwright/test').Page) {
  void page.route('**/api/openapi.json', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        openapi: '3.0.0',
        info: { title: 'Aether' },
        paths: {
          '/api/workloads': { get: { summary: 'List workloads' } },
          '/api/workloads/{name}': { get: { summary: 'Get workload' } },
          '/api/workloads/web': { get: { summary: 'Get web workload' } },
        },
      }),
    }),
  );
}

test.describe('Phases 1631–1680 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1631: platform workload context banner', async ({ page }) => {
    mockPlatform(page);
    await page.goto('/platform?workload=web');
    await expect(page.getByTestId('platform-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('platform-drift-link').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1636: clusters workload context banner', async ({ page }) => {
    mockClusters(page);
    await page.goto('/clusters?workload=web');
    await expect(page.getByTestId('clusters-workload-context')).toContainText('web', { timeout: 15_000 });
    await page.getByTestId('clusters-drift-link').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1641: openapi workload context and route highlight', async ({ page }) => {
    mockOpenApi(page);
    await page.goto('/openapi?workload=web');
    await expect(page.getByTestId('openapi-workload-context')).toContainText('web', { timeout: 10_000 });
    await expect(page.getByTestId('openapi-workload-route-highlight').first()).toBeVisible();
    await page.getByTestId('openapi-workloads-link').first().click();
    await expect(page).toHaveURL(/\/workloads\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1646: breadcrumb workload segment', async ({ page }) => {
    mockPlatform(page);
    await page.goto('/platform?workload=web');
    await expect(page.getByTestId('breadcrumb-workload')).toHaveText('web', { timeout: 10_000 });
    await page.getByTestId('breadcrumb-workload').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1651: rbac workload context openapi link', async ({ page }) => {
    await page.route('**/api/rbac/keys', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/rbac?workload=web');
    await expect(page.getByTestId('rbac-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('rbac-openapi-link').click();
    await expect(page).toHaveURL(/\/openapi\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1656: envs promote prefill from workload query', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/envs?workload=web');
    await expect(page.getByTestId('envs-promote-workload')).toHaveValue('web', { timeout: 10_000 });
  });

  test('phase 1661: metrics workload context cost link', async ({ page }) => {
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
    await page.getByTestId('metrics-cost-link').click();
    await expect(page).toHaveURL(/\/cost\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1666: editor gitops link from workload context', async ({ page }) => {
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
    await page.goto('/editor?workload=web');
    await expect(page.getByTestId('editor-gitops-link')).toBeVisible({ timeout: 15_000 });
    await page.getByTestId('editor-gitops-link').click();
    await expect(page).toHaveURL(/\/gitops\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1671: workload detail openapi quick link', async ({ page }) => {
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
    await page.getByTestId('workload-link-openapi').click();
    await expect(page).toHaveURL(/\/openapi\?workload=web/, { timeout: 10_000 });
  });
});
