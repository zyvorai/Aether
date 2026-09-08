// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1781–1830 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1781: health platform link', async ({ page }) => {
    await page.route('**/api/orchestrator/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { healthy: 1, degraded: 0, unhealthy: 0, unknown: 0, circuits_open: 0 } }),
      }),
    );
    await page.route('**/api/orchestrator/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', runtime: 'kube', health: 'healthy', circuit: 'closed', restart_count: 0 }],
        }),
      }),
    );
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('health-platform-link').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1786: copilot editor link', async ({ page }) => {
    await page.goto('/copilot?workload=web');
    await expect(page.getByTestId('copilot-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('copilot-editor-link').click();
    await expect(page).toHaveURL(/\/editor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1791: events openapi link', async ({ page }) => {
    await page.route('**/api/events/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { total_events: 0, unacknowledged: 0, critical_unacked: 0 } }),
      }),
    );
    await page.route('**/api/events**', (route) => {
      if (route.request().url().includes('/summary')) return route.continue();
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) });
    });
    await page.goto('/events?workload=web');
    await expect(page.getByTestId('events-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('events-openapi-link').click();
    await expect(page).toHaveURL(/\/openapi\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1796: workloads list platform link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'api', status: 'running', runtime: 'kube', image: 'nginx', source: 'cluster' }],
        }),
      }),
    );
    await page.goto('/workloads?workload=web');
    await expect(page.getByTestId('workloads-context-banner')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('workloads-links-more').click();
    await page.getByTestId('workloads-platform-link').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1801: sla drift link', async ({ page }) => {
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
    await page.route('**/api/sla/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.goto('/sla?workload=web');
    await expect(page.getByTestId('sla-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('sla-drift-link').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1806: rbac secrets link', async ({ page }) => {
    await page.route('**/api/rbac/keys', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/rbac?workload=web');
    await expect(page.getByTestId('rbac-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('rbac-context-secrets-link').click();
    await expect(page).toHaveURL(/\/secrets\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1811: workload detail clusters quick link', async ({ page }) => {
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
    await page.getByTestId('workload-link-clusters').click();
    await expect(page).toHaveURL(/\/clusters\?workload=web/, { timeout: 10_000 });
  });
});
