// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 101–150 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 101: AI page tab deep link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/ai?tab=optimize');
    await expect(page.getByTestId('ai-tabs')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('ai-tabs').getByRole('tab', { name: 'Optimization' })).toHaveAttribute('aria-selected', 'true', {
      timeout: 10_000,
    });
  });

  test('phase 102: AI page workload query param', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', runtime: 'podman', status: 'running', image: 'nginx', created_at: new Date().toISOString() }],
        }),
      }),
    );
    await page.goto('/ai?workload=web&tab=optimize');
    await expect(page).toHaveURL(/workload=web/, { timeout: 10_000 });
    await expect(page.getByRole('combobox').nth(1)).toHaveValue('web', { timeout: 10_000 });
  });

  test('phase 103: overview AI engine quick link', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'AI engine' }).first().click();
    await expect(page).toHaveURL(/\/ai/, { timeout: 10_000 });
  });

  test('phase 104: overview validate YAML opens modal', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/');
    await page.getByRole('button', { name: 'Validate YAML' }).click();
    await expect(page).toHaveURL(/\/workloads/, { timeout: 10_000 });
    await expect(page.getByRole('heading', { name: 'Validate Workload YAML' })).toBeVisible({ timeout: 10_000 });
  });

  test('phase 105: command palette opens AI engine', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('ai engine');
    await page.getByRole('button', { name: 'Open AI engine' }).click();
    await expect(page).toHaveURL(/\/ai/, { timeout: 10_000 });
  });

  test('phase 106: command palette validate yaml', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('validate yaml');
    await page.getByRole('button', { name: 'Validate workload YAML' }).click();
    await expect(page).toHaveURL(/\/workloads/, { timeout: 10_000 });
    await expect(page.getByRole('heading', { name: 'Validate Workload YAML' })).toBeVisible({ timeout: 10_000 });
  });

  test('phase 107: platform cilium cluster browser link', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            kubernetes: {
              cilium: {
                cluster: 'prod-east',
                cni: 'cilium',
                egress_mode: 'strict',
                cilium_daemonset_ready: true,
                metrics_server: true,
                managed_policies: [],
              },
            },
          },
        }),
      }),
    );
    await page.route('**/api/platform/recommendations', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { items: [] } }),
      }),
    );
    await page.goto('/platform');
    await page.getByTestId('platform-cilium-cluster-link').click();
    await expect(page).toHaveURL(/\/clusters.*tab=network|tab=network.*\/clusters/, { timeout: 10_000 });
  });

  test('phase 108: gitops external repo link', async ({ page }) => {
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            configured: true,
            repo_url: 'https://github.com/example/aether-gitops',
            branch: 'main',
          },
        }),
      }),
    );
    await page.goto('/gitops');
    const link = page.getByTestId('gitops-repo-external-link');
    await expect(link).toBeVisible({ timeout: 10_000 });
    await expect(link).toHaveAttribute('href', 'https://github.com/example/aether-gitops');
  });

  test('phase 109: metrics cluster browser link', async ({ page }) => {
    await page.route('**/api/metrics', (route) => route.fulfill({ status: 200, body: '# HELP test\n' }));
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { integrations: {} } }),
      }),
    );
    await page.route('**/api/observability/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            prometheus_configured: false,
            api_http_requests_total: 0,
            migrations_total: 0,
            migration_rollbacks_total: 0,
            workloads_running: {},
          },
        }),
      }),
    );
    await page.goto('/metrics');
    await page.getByTestId('metrics-cluster-browser-link').click();
    await expect(page).toHaveURL(/\/clusters.*tab=network|tab=network.*\/clusters/, { timeout: 10_000 });
  });

  test('phase 110: copilot suggestion chips', async ({ page }) => {
    await page.goto('/copilot');
    await expect(page.getByTestId('copilot-suggestions')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('copilot-suggestion').first()).toBeVisible();
  });

  test('phase 111: backups create modal testid', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      }),
    );
    await page.goto('/backups');
    await page.getByRole('button', { name: 'Create backup' }).click();
    await expect(page.getByTestId('backup-create-modal')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 112: deps add form testid', async ({ page }) => {
    await page.route('**/api/dependencies', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            nodes: [],
            edges: [],
            startup_order: [],
            issues: [],
            stats: {
              total_workloads: 0,
              total_edges: 0,
              root_workloads: 0,
              leaf_workloads: 0,
              max_depth: 0,
              has_cycles: false,
            },
          },
        }),
      }),
    );
    await page.goto('/deps');
    await expect(page.getByTestId('deps-add-form')).toBeVisible({ timeout: 10_000 });
  });
});
