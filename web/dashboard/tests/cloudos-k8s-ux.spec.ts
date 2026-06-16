// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

function mockKubeWorkloads(page: import('@playwright/test').Page) {
  return page.route('**/api/workloads', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        data: [
          {
            name: 'api',
            status: 'running',
            runtime: 'kube',
            image: 'nginx:latest',
            source: 'cluster',
            namespace: 'default',
          },
        ],
      }),
    }),
  );
}

test.describe('CloudOS Kubernetes UX', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('applications page loads with kube workloads', async ({ page }) => {
    await mockKubeWorkloads(page);
    await page.goto('/applications');
    await expect(page.getByTestId('applications-page')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText('api')).toBeVisible();
  });

  test('activity monitor page loads', async ({ page }) => {
    await mockKubeWorkloads(page);
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { connected: true, clusters: [{ name: 'active-client', reachable: true }] },
        }),
      }),
    );
    await page.route('**/api/events', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.route('**/api/cluster/metrics**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { pods: [] } }),
      }),
    );
    await page.goto('/activity');
    await expect(page.getByTestId('activity-monitor-page')).toBeVisible({ timeout: 15_000 });
  });

  test('security center page loads', async ({ page }) => {
    await page.route('**/api/intelligence/threats', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { score: 85, threats: [], last_scan: null },
        }),
      }),
    );
    await page.route('**/api/secrets', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.route('**/api/security/sbom', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { metadata: { generated_at: new Date().toISOString(), packages: 0 } } }),
      }),
    );
    await page.route('**/api/security/images', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.route('**/api/ecosystem/packetwolf/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { configured: false, reachable: false } }),
      }),
    );
    await page.goto('/security');
    await expect(page.getByTestId('security-center-page')).toBeVisible({ timeout: 15_000 });
  });

  test('helm catalog page loads curated charts', async ({ page }) => {
    await page.route('**/api/helm/catalog', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              name: 'nginx',
              chart: 'nginx',
              repo: 'https://charts.bitnami.com/bitnami',
              description: 'NGINX web server',
              category: 'Web',
            },
          ],
        }),
      }),
    );
    await page.goto('/helm');
    await expect(page.getByTestId('helm-catalog-page')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText('nginx').first()).toBeVisible();
  });
});
