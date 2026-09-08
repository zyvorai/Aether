// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1331–1380 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });


  test('phase 1341: events workload context cross-links drift', async ({ page }) => {
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
    await expect(page.getByTestId('events-workload-context')).toContainText('web');
    await page.getByTestId('events-drift-link').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1346: alerts workload context metrics link', async ({ page }) => {
    await page.route('**/api/alerts/status**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { rules: [], channels: [], queue_depth: 0 } }),
      }),
    );
    await page.goto('/alerts?workload=web');
    await expect(page.getByTestId('alerts-workload-context')).toContainText('web');
    await page.getByTestId('alerts-metrics-link').click();
    await expect(page).toHaveURL(/\/metrics\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1350: health workload context gitops link in detail', async ({ page }) => {
    await page.route('**/api/orchestrator/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { healthy: 1, degraded: 0, unhealthy: 0, unknown: 0, circuits_open: 0 },
        }),
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
    await page.route('**/api/health/web', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total_checks: 1, ready_checks: 1, uptime_percent: 100, last_state: 'ready' },
        }),
      }),
    );
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-workload-context')).toContainText('web');
    await page.getByRole('row', { name: /web/ }).click();
    await page.getByTestId('health-detail-panel').getByTestId('health-gitops-link').click();
    await expect(page).toHaveURL(/\/gitops\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1356: copilot workload context audit link', async ({ page }) => {
    await page.goto('/copilot?workload=web');
    await expect(page.getByTestId('copilot-workload-context')).toContainText('web');
    await page.getByTestId('copilot-audit-link').click();
    await expect(page).toHaveURL(/\/audit\?workload=web/, { timeout: 10_000 });
  });
});
