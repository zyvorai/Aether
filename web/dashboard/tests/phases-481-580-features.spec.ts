// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 481–580 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 481: intelligence threats panel', async ({ page }) => {
    await page.route('**/api/intelligence/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.route('**/api/intelligence/threats', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { threats: [{ workload: 'web', severity: 'high', category: 'network', reason: 'test', score: 0.8, detected_at: 'now' }] },
        }),
      }),
    );
    await page.goto('/intelligence?tab=threats');
    await expect(page.getByTestId('intelligence-threats-panel')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 482: copilot send button', async ({ page }) => {
    await page.goto('/copilot');
    await expect(page.getByTestId('copilot-send-button')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 483: editor export helm button', async ({ page }) => {
    await page.goto('/editor');
    await expect(page.getByTestId('editor-export-helm')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 484: envs list testid', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'dev', tier: 'development', workloads: {}, variables: {}, updated_at: new Date().toISOString() }],
        }),
      }),
    );
    await page.goto('/envs');
    await expect(page.getByTestId('envs-list')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 485: fleet clusters link', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { enabled: true, connected: true, cluster_count: 1, healthy_clusters: 1, workload_count: 0, clusters: [] } }),
      }),
    );
    await page.route('**/api/server', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { integrations: {} } }) }),
    );
    await page.goto('/fleet');
    await page.getByTestId('fleet-clusters-link').click();
    await expect(page).toHaveURL(/\/clusters/, { timeout: 10_000 });
  });

  test('phase 486: backups create button', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/backups');
    await expect(page.getByTestId('backups-create-button')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 487: backups audit link', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/backups');
    await page.getByTestId('backups-audit-link').click();
    await expect(page).toHaveURL(/\/audit/, { timeout: 10_000 });
  });

  test('phase 488: alerts events link', async ({ page }) => {
    await page.route('**/api/alerts/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { channels: [], rules: [] } }) }),
    );
    await page.route('**/api/webhooks/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/alerts');
    await page.getByTestId('alerts-events-link').first().click();
    await expect(page).toHaveURL(/\/events/, { timeout: 10_000 });
  });

  test('phase 489: cost estimate form', async ({ page }) => {
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
    await page.goto('/cost');
    await expect(page.getByTestId('cost-estimate-form')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 490: scheduler suggestions panel', async ({ page }) => {
    await page.route('**/api/scheduler/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/scheduler');
    await expect(page.getByTestId('scheduler-suggestions')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 491: ai analyze panel deep link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/ai?tab=analyze');
    await expect(page.getByTestId('ai-analyze-panel')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 492: deps stats panel', async ({ page }) => {
    await page.route('**/api/dependencies', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            nodes: ['a'],
            edges: [],
            startup_order: ['a'],
            issues: [],
            stats: { total_workloads: 1, total_edges: 0, root_workloads: 1, leaf_workloads: 1, max_depth: 0, has_cycles: false },
          },
        }),
      }),
    );
    await page.goto('/deps');
    await expect(page.getByTestId('deps-stats-panel')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 493: command palette opens metrics', async ({ page }) => {
    await page.route('**/api/observability/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { grafana_configured: false, prometheus_configured: false, packetwolf_configured: false } }),
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
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('metrics');
    await page.getByRole('button', { name: 'Open metrics & Grafana' }).click();
    await expect(page).toHaveURL(/\/metrics/, { timeout: 10_000 });
  });

  test('phase 494: sla events link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/sla');
    await page.getByRole('link', { name: 'SLA events →' }).click();
    await expect(page).toHaveURL(/category=sla/, { timeout: 10_000 });
  });

  test('phase 495: plugins list testid', async ({ page }) => {
    await page.route('**/api/plugins', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'demo', version: '1.0', runtime_kind: 'podman', capabilities: ['run'] }],
        }),
      }),
    );
    await page.goto('/plugins');
    await expect(page.getByTestId('plugins-list')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 496: affinity recommend panel', async ({ page }) => {
    await page.route('**/api/affinity/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ runtime: 'kube', composite_score: 0.9, cost_score: 0.8, performance_score: 0.9, reliability_score: 0.9, availability_score: 0.9 }],
        }),
      }),
    );
    await page.goto('/affinity');
    await expect(page.getByTestId('affinity-recommend-panel')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 497: events severity filter testid', async ({ page }) => {
    await page.route('**/api/events**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/events/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { total_events: 0, unacknowledged: 0, critical_unacked: 0 } }),
      }),
    );
    await page.goto('/events');
    await expect(page.getByTestId('events-severity-filter')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 498: platform audit link', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { integrations: {} } }) }),
    );
    await page.route('**/api/platform/recommendations', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { items: [] } }) }),
    );
    await page.goto('/platform');
    await page.getByTestId('platform-audit-link').click();
    await expect(page).toHaveURL(/\/audit/, { timeout: 10_000 });
  });
});
