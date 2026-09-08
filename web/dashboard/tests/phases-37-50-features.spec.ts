// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 37–50 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 37: events workload links to detail', async ({ page }) => {
    await page.route('**/api/events**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              timestamp: new Date().toISOString(),
              severity: 'warning',
              category: 'health',
              title: 'Health degraded',
              message: 'Probe failing',
              workload: 'web',
              source: 'orchestrator',
              acknowledged: false,
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
          data: { total_events: 1, unacknowledged: 1, critical_unacked: 0 },
        }),
      }),
    );

    await page.goto('/events');
    await page.getByRole('link', { name: 'web' }).click();
    await expect(page).toHaveURL(/workload=web/, { timeout: 10_000 });
  });

  test('phase 38–39: fleet cluster deep link and workload stat', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            connected: true,
            cluster_count: 1,
            healthy_clusters: 1,
            workload_count: 3,
            clusters: [{ name: 'prod', reachable: true, server: 'https://k8s.example', version: '1.29' }],
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
    await page.getByTestId('fleet-cluster-prod').click();
    await expect(page).toHaveURL(/cluster=prod/, { timeout: 10_000 });
  });

  test('phase 40: platform recommendation dismiss', async ({ page }) => {
    await page.route('**/api/platform/recommendations', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            items: [
              {
                id: 'opa-missing',
                title: 'Configure OPA',
                category: 'policy',
                severity: 'warn',
                detail: 'OPA not configured',
                action: 'Set AETHER_OPA_URL',
              },
            ],
          },
        }),
      }),
    );
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            safety: {
              mutation_confirm_required: false,
              mutation_confirm_header: 'X-Aether-Confirm',
              mutation_confirm_values: ['1'],
            },
            opa: { configured: false },
            oidc: { enabled: false },
          },
        }),
      }),
    );

    await page.goto('/platform');
    await expect(page.getByText('Configure OPA')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('platform-rec-dismiss-opa-missing').click();
    await expect(page.getByText('Configure OPA')).not.toBeVisible({ timeout: 5_000 });
  });

  test('phase 41: openapi method filter and copy path', async ({ page }) => {
    await page.route('**/api/openapi.json', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          openapi: '3.0.0',
          info: { title: 'Aether', version: '0.3.0' },
          paths: {
            '/api/workloads': { get: { summary: 'List workloads' }, post: { summary: 'Create workload' } },
            '/api/health/{workload}': { get: { summary: 'Health summary' } },
          },
        }),
      }),
    );

    await page.goto('/openapi');
    await expect(page.getByTestId('openapi-method-filter')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('openapi-method-filter').selectOption('GET');
    await expect(page.getByRole('cell', { name: '/api/workloads' })).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('cell', { name: 'POST' })).not.toBeVisible();
    await page.getByTestId('openapi-copy-GET-/api/workloads').click();
    await expect(page.getByText('Copied')).toBeVisible({ timeout: 5_000 });
  });

  test('phase 42: compose deploy result links workloads', async ({ page }) => {
    await page.route('**/api/compose/validate', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { valid: true, workload_count: 1, deploy_order: ['web'] },
        }),
      }),
    );
    await page.route('**/api/policy/check', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { passed: true, policies_evaluated: 1, violations: [], warnings: [] } }),
      }),
    );
    await page.route('**/api/compose/up', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { deployed: ['compose-web'], count: 1 } }),
      }),
    );

    await page.goto('/compose');
    await page.getByRole('button', { name: 'Validate compose' }).click();
    await page.getByRole('button', { name: 'Deploy stack' }).click();
    await expect(page.getByTestId('compose-deploy-result')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('compose-deploy-result').getByRole('link', { name: 'compose-web' }).click();
    await expect(page).toHaveURL(/workload=compose-web/, { timeout: 10_000 });
  });

  test('phase 43: overview quick links include drift and fleet', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('button', { name: 'Drift detection' })).toBeVisible({ timeout: 10_000 });
    await page.getByRole('button', { name: 'Fleet overview' }).click();
    await expect(page).toHaveURL(/\/fleet/, { timeout: 10_000 });
  });

  test('phase 44: backup restore modal', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              filename: 'snapshot-2026.json',
              created_at: new Date().toISOString(),
              workload_count: 2,
              aether_version: '0.3.0',
            },
          ],
        }),
      }),
    );

    await page.goto('/backups');
    await page.getByRole('button', { name: 'Restore' }).first().click();
    await expect(page.getByTestId('backup-restore-modal')).toBeVisible({ timeout: 10_000 });
  });
});
