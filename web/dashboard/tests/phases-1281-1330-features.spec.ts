// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1281–1330 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1281: gitops workload context links to trust', async ({ page }) => {
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { configured: true, repo_url: 'https://git.example/repo', branch: 'main', last_sync: 'now' },
        }),
      }),
    );
    await page.goto('/gitops?workload=web');
    await expect(page.getByTestId('gitops-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('gitops-trust-link').click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1282: metrics workload alias shows context and trust link', async ({ page }) => {
    await page.route('**/api/metrics', (route) =>
      route.fulfill({ status: 200, contentType: 'text/plain', body: '# HELP aether_workloads\n' }),
    );
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { integrations: { grafana_url: 'http://grafana.local', prometheus_url: 'http://prom.local' } },
        }),
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
            pricingSource: 'test',
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
        body: JSON.stringify({
          success: true,
          data: {
            grafana_configured: true,
            prometheus_configured: true,
            workloads_running: { kube: 1 },
          },
        }),
      }),
    );
    await page.goto('/metrics?workload=web');
    await expect(page.getByTestId('metrics-workload-context')).toContainText('web', { timeout: 15_000 });
    await page.getByTestId('metrics-trust-link').click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1283: deps workload context links to alerts', async ({ page }) => {
    await page.route('**/api/dependencies', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            nodes: ['web'],
            edges: [],
            startup_order: ['web'],
            issues: [],
            stats: {
              total_workloads: 1,
              total_edges: 0,
              root_workloads: 1,
              leaf_workloads: 1,
              max_depth: 0,
              has_cycles: false,
            },
          },
        }),
      }),
    );
    await page.goto('/deps?workload=web');
    await expect(page.getByTestId('deps-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('deps-alerts-link').click();
    await expect(page).toHaveURL(/\/alerts\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1284: workload events tab cross-links to trust', async ({ page }) => {
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
    await page.route('**/api/events**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/workloads?workload=web&tab=events');
    await expect(page.getByTestId('workload-events-cross-links')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('workload-events-trust-link').click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1285: deploy success panel links to events', async ({ page }) => {
    const unique = `evt-${Date.now().toString(36)}`;
    await page.route('**/api/validate', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { valid: true, workload_name: unique, errors: [] } }),
      }),
    );
    await page.route('**/api/policy/check', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { passed: true, policies_evaluated: 1, violations: [], warnings: [] },
        }),
      }),
    );
    await page.route('**/api/workloads', (route) => {
      if (route.request().method() === 'POST') {
        return route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({ success: true, data: `Workload ${unique} created` }),
        });
      }
      if (route.request().method() === 'GET') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: [
              {
                name: unique,
                runtime: 'kube',
                image: 'docker.io/library/httpd:latest',
                status: 'running',
                created_at: new Date().toISOString(),
                source: 'aether',
              },
            ],
          }),
        });
      }
      return route.continue();
    });

    await page.goto('/workloads?deploy=1&source=aether');
    const dialog = page.getByRole('dialog', { name: 'Deploy New Workload' });
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    const editor = dialog.getByRole('textbox', { name: /workload yaml/i });
    await editor.click();
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.insertText(`apiVersion: aether/v1
kind: Workload
metadata:
  name: ${unique}
runtime:
  preferred: kube
  allow: [kube]
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 100m
  memory: 128Mi
network:
  service: true
  ports:
    - containerPort: 80
`);
    await dialog.getByTestId('workloads-deploy-submit').click({ force: true });
    await expect(dialog.getByTestId('deploy-success-panel')).toBeVisible({ timeout: 15_000 });
    await dialog.getByTestId('deploy-success-events-link').click();
    await expect(page).toHaveURL(new RegExp(`/events\\?workload=${unique}`), { timeout: 10_000 });
  });

  test('phase 1286: palette opens gitops for workload', async ({ page }) => {
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
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('gitops web');
    await page.getByRole('button', { name: /GitOps sync: web/i }).click();
    await expect(page).toHaveURL(/\/gitops\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1287: envs workload banner links to trust', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/envs?workload=web');
    await expect(page.getByTestId('envs-workload-banner')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('envs-trust-link').click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });
});
