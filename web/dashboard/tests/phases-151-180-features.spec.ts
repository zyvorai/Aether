// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 151–180 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 151: affinity tab deep link', async ({ page }) => {
    await page.route('**/api/affinity/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/affinity?tab=matrix');
    await expect(page.getByTestId('affinity-tabs').getByRole('button', { name: 'matrix' })).toHaveClass(/bg-aether/, {
      timeout: 10_000,
    });
  });

  test('phase 152: overview cost estimation quick link', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Cost estimation' }).click();
    await expect(page).toHaveURL(/\/cost/, { timeout: 10_000 });
  });

  test('phase 153: command palette opens runtime affinity', async ({ page }) => {
    await page.route('**/api/affinity/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('affinity');
    await page.getByRole('button', { name: 'Open runtime affinity' }).click();
    await expect(page).toHaveURL(/\/affinity/, { timeout: 10_000 });
  });

  test('phase 154: alerts add channel form testid', async ({ page }) => {
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
    await page.getByRole('button', { name: 'Add webhook' }).click();
    await expect(page.getByTestId('alerts-add-channel-form')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 155: envs create modal testid', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
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
    await page.getByRole('button', { name: 'Create environment' }).click();
    await expect(page.getByTestId('envs-create-modal')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 156: cost chargeback workload link', async ({ page }) => {
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 99,
            totalSpotMonthlyUsd: 20,
            tco36MonthsUsd: 3500,
            pricingSource: 'aws',
            region: 'us-east-1',
            lines: [{ workload: 'api', owner: 'team', project: 'demo', monthlyUsd: 99 }],
          },
        }),
      }),
    );
    await page.goto('/cost');
    await page.getByRole('link', { name: 'api' }).click();
    await expect(page).toHaveURL(/workload=api/, { timeout: 10_000 });
  });

  test('phase 157: editor workload query context', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', runtime: 'kubernetes', status: 'running', image: 'nginx:latest', created_at: new Date().toISOString() }],
        }),
      }),
    );
    await page.goto('/editor?workload=web');
    await expect(page.getByTestId('editor-workload-context')).toContainText('web', { timeout: 10_000 });
  });

  test('phase 158: scheduler affinity link', async ({ page }) => {
    await page.route('**/api/scheduler/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/scheduler');
    await page.getByRole('link', { name: 'Runtime affinity →' }).click();
    await expect(page).toHaveURL(/\/affinity/, { timeout: 10_000 });
  });

  test('phase 159: sla add form testid', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { authenticated: true, role: 'operator', username: 'admin' } }),
      }),
    );
    await page.goto('/sla');
    await expect(page.getByTestId('sla-add-form')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 160: fleet healthy stat opens health monitor', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            enabled: true,
            connected: true,
            cluster_count: 2,
            healthy_clusters: 2,
            workload_count: 5,
            clusters: [],
          },
        }),
      }),
    );
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { integrations: {} } }),
      }),
    );
    await page.goto('/fleet');
    await page.getByRole('button', { name: 'Healthy' }).click();
    await expect(page).toHaveURL(/\/health-monitor/, { timeout: 10_000 });
  });

  test('phase 161: events clear filters', async ({ page }) => {
    await page.route('**/api/events**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              title: 'Test',
              message: 'msg',
              severity: 'warning',
              category: 'health',
              timestamp: new Date().toISOString(),
              workload: 'web',
            },
          ],
        }),
      }),
    );
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
    await page.goto('/events?category=health&workload=web');
    await page.getByTestId('events-clear-filters').click();
    await expect(page).toHaveURL(/\/events(?:\?|$)/, { timeout: 10_000 });
    await expect(page).not.toHaveURL(/category=health/);
  });

  test('phase 162: plugins register form testid', async ({ page }) => {
    await page.route('**/api/plugins', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { authenticated: true, role: 'operator', username: 'admin' } }),
      }),
    );
    await page.goto('/plugins');
    await expect(page.getByTestId('plugins-register-form')).toBeVisible({ timeout: 10_000 });
  });
});
