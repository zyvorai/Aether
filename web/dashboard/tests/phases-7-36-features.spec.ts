// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 7–36 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 7: command palette shows setup commands', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            version: '0.3.0',
            safety: {
              mutation_confirm_required: false,
              mutation_confirm_header: 'X-Aether-Confirm',
              mutation_confirm_values: ['1'],
            },
            opa: { configured: false, enforce: false },
            oidc: { enabled: false },
            workload_state: { backend: 'local-json', configured: true },
          },
        }),
      }),
    );
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { configured: false } }),
      }),
    );
    await page.reload();
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('configure OPA');
    await expect(page.getByRole('button', { name: /Setup: Policy Check/i })).toBeVisible({ timeout: 10_000 });
    await page.getByRole('button', { name: /Setup: Policy Check/i }).click();
    await expect(page).toHaveURL(/\/platform/, { timeout: 10_000 });
  });

  test('phase 8–11: drift bulk scan and open workload link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            { name: 'web', runtime: 'kube', status: 'running', source: 'aether' },
            { name: 'api', runtime: 'kube', status: 'running', source: 'aether' },
          ],
        }),
      }),
    );
    await page.route('**/api/drift/web', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            workload_name: 'web',
            has_drift: true,
            severity: 'medium',
            drifts: [{ field: 'replicas', expected: '2', actual: '1', severity: 'medium', category: 'spec' }],
            reconciliation_plan: [{ action_type: 'scale', description: 'Scale to 2', requires_restart: false, risk: 'low' }],
          },
        }),
      }),
    );
    await page.route('**/api/drift/api', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { workload_name: 'api', has_drift: false, severity: 'none', drifts: [], reconciliation_plan: [] },
        }),
      }),
    );

    await page.goto('/drift');
    await page.getByTestId('drift-bulk-scan').click();
    await expect(page.getByTestId('drift-bulk-summary')).toContainText('1 with drift', { timeout: 15_000 });
    await page.getByTestId('drift-open-workload').click();
    await expect(page).toHaveURL(/workload=web.*tab=drift|tab=drift.*workload=web/, { timeout: 10_000 });
  });

  test('phase 12: health rolling update controls', async ({ page }) => {
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { authenticated: true, role: 'operator', username: 'admin' },
        }),
      }),
    );
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
          data: {
            total_checks: 10,
            ready_checks: 10,
            uptime_percent: 99.9,
            last_state: 'ready',
            last_restart_count: 0,
          },
        }),
      }),
    );

    await page.goto('/health-monitor');
    await expect(page.getByRole('cell', { name: 'web', exact: true })).toBeVisible({ timeout: 10_000 });
    await page.getByRole('cell', { name: 'web', exact: true }).click();
    await expect(page.getByTestId('health-rolling-update')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 13: policy OPA setup banner', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { opa: { configured: false, enforce: false } },
        }),
      }),
    );
    await page.goto('/policy');
    await expect(page.getByTestId('policy-opa-setup-banner')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('policy-opa-setup-banner').getByRole('button', { name: 'Open Platform' }).click();
    await expect(page).toHaveURL(/\/platform/, { timeout: 10_000 });
  });

  test('phase 14: metrics prometheus setup banner', async ({ page }) => {
    await page.route('**/api/metrics', (route) => route.fulfill({ status: 200, body: '# HELP test\n' }));
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { integrations: {} } }),
      }),
    );
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 0,
            totalSpotMonthlyUsd: 0,
            tco36MonthsUsd: 0,
            pricingSource: 'aws',
            region: 'us-east-1',
            lines: [],
          },
        }),
      }),
    );
    await page.route('**/api/observability/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { prometheus_configured: false, api_http_requests_total: 0, migrations_total: 0, migration_rollbacks_total: 0, workloads_running: {} } }),
      }),
    );

    await page.goto('/metrics');
    await expect(page.getByTestId('metrics-prom-setup-banner')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 15: cost page fleet chargeback preview', async ({ page }) => {
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 42.5,
            totalSpotMonthlyUsd: 10,
            tco36MonthsUsd: 1530,
            pricingSource: 'aws',
            region: 'us-east-1',
            lines: [{ workload: 'web', owner: 'team-a', project: 'demo', monthlyUsd: 42.5 }],
          },
        }),
      }),
    );
    await page.goto('/cost');
    await expect(page.getByTestId('cost-fleet-chargeback')).toContainText('Fleet chargeback', { timeout: 10_000 });
  });

  test('phase 16: audit re-verify button', async ({ page }) => {
    await page.route('**/api/audit', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            summary: { total_events: 1, successes: 1, failures: 0, unique_workloads: 1 },
            recent_events: [{ id: '1', timestamp: new Date().toISOString(), action: 'deploy', workload: 'web', result: 'success', message: 'ok' }],
          },
        }),
      }),
    );
    await page.route('**/api/audit/verify', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { integrity: 'VERIFIED', verified: 1, total: 1, tampered: 0, tampered_events: [] },
        }),
      }),
    );

    await page.goto('/audit');
    await expect(page.getByTestId('audit-reverify')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 17: scheduler placement links to workload', async ({ page }) => {
    await page.route('**/api/scheduler/utilization', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/scheduler/optimize', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/scheduler/placements', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ workload_name: 'web', runtime: 'kube', cpu_reserved: 500, memory_reserved_mb: 512, placed_at: new Date().toISOString() }],
        }),
      }),
    );

    await page.goto('/scheduler');
    await page.getByRole('link', { name: 'web' }).click();
    await expect(page).toHaveURL(/workload=web/, { timeout: 10_000 });
  });
});
