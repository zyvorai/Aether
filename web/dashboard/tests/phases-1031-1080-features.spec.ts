// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';
import { clickTestId, waitForOverviewReady } from './helpers/overview';

test.describe('Phases 1031–1080 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1031: overview events stat applies severity when unacknowledged', async ({ page }) => {
    await page.route('**/api/events/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total_events: 12, unacknowledged: 3, critical_unacked: 1 },
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
    await page.goto('/');
    await expect(page.getByTestId('overview-events-stat')).toContainText('12', { timeout: 15_000 });
    await clickTestId(page, 'overview-events-stat');
    await expect(page).toHaveURL(/severity=warning/, { timeout: 10_000 });
  });

  test('phase 1032: overview platform quick link testid', async ({ page }) => {
    await waitForOverviewReady(page);
    await clickTestId(page, 'overview-platform-quick-link');
    await expect(page).toHaveURL(/\/platform/, { timeout: 10_000 });
  });

  test('phase 1033: events workload context banner', async ({ page }) => {
    await page.route('**/api/events**', (route) => {
      if (route.request().url().includes('/summary')) {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: { total_events: 0, unacknowledged: 0, critical_unacked: 0 },
          }),
        });
      }
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      });
    });
    await page.goto('/events?workload=web');
    await expect(page.getByTestId('events-workload-context')).toContainText('web', { timeout: 10_000 });
  });

  test('phase 1034: health workload context banner', async ({ page }) => {
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
          data: [
            {
              name: 'web',
              health: 'healthy',
              runtime: 'kube',
              circuit: 'closed',
              restart_count: 0,
            },
          ],
        }),
      }),
    );
    await page.route('**/api/health/web', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            total_checks: 1,
            ready_checks: 1,
            uptime_percent: 100,
            last_state: 'healthy',
            last_restart_count: 0,
          },
        }),
      }),
    );
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-workload-context')).toContainText('web', { timeout: 10_000 });
  });




  test('phase 1038: workload detail alerts opens alerts page', async ({ page }) => {
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
    await page.route('**/api/workloads/web/snapshots', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/alerts/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { channels: [], rules: [] } }),
      }),
    );
    await page.route('**/api/webhooks/queue', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-alerts').click();
    await expect(page).toHaveURL(/\/alerts\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1039: envs search query context banner', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/envs?q=web');
    await expect(page.getByTestId('envs-workload-context')).toContainText('web', { timeout: 10_000 });
  });
});
