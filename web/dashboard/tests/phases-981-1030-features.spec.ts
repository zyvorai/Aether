// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

const workloadRow = {
  name: 'web',
  status: 'running',
  runtime: 'kube',
  image: 'nginx:latest',
  source: 'cluster',
};

async function mockWorkloadDetail(page: import('@playwright/test').Page) {
  await page.route('**/api/workloads', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true, data: [workloadRow] }),
    }),
  );
  await page.route('**/api/workloads/web/snapshots', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
  );
}

test.describe('Phases 981–1030 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 981: overview runtime fabric count is not doubled', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            { name: 'web', status: 'running', runtime: 'kube', image: 'nginx', source: 'cluster' },
            { name: 'api', status: 'running', runtime: 'kube', image: 'nginx', source: 'aether' },
          ],
        }),
      }),
    );
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            enabled: true,
            connected: true,
            cluster_count: 1,
            healthy_clusters: 1,
            workload_count: 99,
            clusters: [{ name: 'dev', reachable: true }],
          },
        }),
      }),
    );
    await page.goto('/');
    await expect(page.getByTestId('overview-runtime-fabric-count')).toHaveText('2', { timeout: 10_000 });
  });

  test('phase 982: intelligence workload context banner', async ({ page }) => {
    await page.route('**/api/intelligence/predictions', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            generated_at: '2026-01-01T00:00:00Z',
            fleet_risk_score: 0.2,
            predictions: [{ workload: 'web', risk_level: 'low', risk_score: 0.1, predictions: [] }],
          },
        }),
      }),
    );
    await page.route('**/api/intelligence/threats', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { threats: [] } }) }),
    );
    await page.route('**/api/intelligence/cost-optimize', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { total_potential_savings_pct: 0, recommendations: [] } }),
      }),
    );
    await page.route('**/api/intelligence/evolution/status', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { workloads: [] } }) }),
    );
    await page.goto('/intelligence?workload=web');
    await expect(page.getByTestId('intelligence-workload-context')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('intelligence-workload-highlight')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 983: workload detail envs link scoped', async ({ page }) => {
    await mockWorkloadDetail(page);
    await page.route('**/api/environments', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-envs').click();
    await expect(page).toHaveURL(/\/envs\?q=web/, { timeout: 10_000 });
  });

  test('phase 984: workload detail alerts link opens alerts filtered', async ({ page }) => {
    await mockWorkloadDetail(page);
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

  test('phase 985: affinity workload context banner', async ({ page }) => {
    await page.route('**/api/affinity/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ runtime: 'kube', composite_score: 0.9, confidence: 0.8, success_rate: 0.9, total_deployments: 1 }],
        }),
      }),
    );
    await page.goto('/affinity?workload=web');
    await expect(page.getByTestId('affinity-workload-context')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 986: overview compose quick link testid', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-compose-quick-link').click();
    await expect(page).toHaveURL(/\/compose/, { timeout: 10_000 });
  });

  test('phase 987: overview cost quick link testid', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-cost-quick-link').click();
    await expect(page).toHaveURL(/\/cost/, { timeout: 10_000 });
  });

  test('phase 988: overview validate quick link testid', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-validate-quick-link').click();
    await expect(page).toHaveURL(/\/workloads/, { timeout: 10_000 });
    await expect(page.getByRole('dialog', { name: 'Validate Workload YAML' })).toBeVisible({
      timeout: 10_000,
    });
  });

  test('phase 989: palette events health deep link', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    const paletteInput = page.getByPlaceholder('Search pages, workloads, and actions…');
    await expect(paletteInput).toBeVisible({ timeout: 10_000 });
    await paletteInput.fill('events health');
    await page.getByRole('button', { name: 'Open health events' }).click();
    await expect(page).toHaveURL(/category=health/, { timeout: 10_000 });
  });

  test('phase 990: workloads update modal submit testid', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx', source: 'aether' }],
        }),
      }),
    );
    await page.goto('/workloads?source=aether');
    await page.getByTitle('Update spec').click();
    const dialog = page.getByRole('dialog', { name: 'Update: web' });
    await expect(dialog.getByTestId('workloads-update-submit')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 991: copilot query param prefills input', async ({ page }) => {
    await page.goto('/copilot?q=Why%20is%20web%20unhealthy%3F');
    await expect(page.getByTestId('copilot-input')).toHaveValue('Why is web unhealthy?', { timeout: 10_000 });
  });
});
