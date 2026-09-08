// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 881–930 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 881: plugins search URL sync', async ({ page }) => {
    await page.route('**/api/plugins', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/plugins?q=podman');
    await expect(page.getByPlaceholder('Search plugins…')).toHaveValue('podman', { timeout: 10_000 });
  });

  test('phase 882: envs search URL sync', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/envs?q=staging');
    await expect(page.getByPlaceholder('Search environments…')).toHaveValue('staging', { timeout: 10_000 });
  });

  test('phase 883: confidential workload highlight', async ({ page }) => {
    await page.route('**/api/confidential/capabilities', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { tee_available: false, runtimes: [] } }),
      }),
    );
    await page.route('**/api/confidential/fleet', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              workload: 'web',
              runtime: 'kubevirt',
              tee: 'sev',
              trust: { composite: 0.9, attestation_score: 0.9 },
              image_in_catalog: true,
            },
          ],
        }),
      }),
    );
    await page.route('**/api/confidential/trust-score', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/confidential/sovereign/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { offline_attestation: false } }),
      }),
    );
    await page.route('**/api/confidential/images', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/confidential/kata/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            supported_runtime_classes: [],
            operator_requirements: [],
            hypervisor: 'none',
            host_tee: { available: false },
            placement_ready: false,
            notes: [],
          },
        }),
      }),
    );
    await page.route('**/api/confidential/intelligence', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { generated_at: '2026-01-01T00:00:00Z', workloads: [], fleet_trust_avg: 0, critical_count: 0 },
        }),
      }),
    );
    await page.goto('/confidential?workload=web');
    await expect(page.getByTestId('confidential-workload-context')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('confidential-workload-highlight')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 884: deps workload context', async ({ page }) => {
    await page.route('**/api/dependencies', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            nodes: ['web', 'db'],
            edges: [{ from: 'web', to: 'db' }],
            startup_order: ['db', 'web'],
            stats: { total_workloads: 2, total_edges: 1, root_workloads: 1, leaf_workloads: 1, max_depth: 1, has_cycles: false },
            issues: [],
          },
        }),
      }),
    );
    await page.goto('/deps?workload=web');
    await expect(page.getByTestId('deps-workload-context')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('deps-workload-highlight')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 885: workload detail metrics link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx:latest' }],
        }),
      }),
    );
    await page.route('**/api/workloads/web/snapshots', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-metrics').click();
    await expect(page).toHaveURL(/\/metrics\?q=web/, { timeout: 10_000 });
  });

  test('phase 886: workload detail fleet link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx:latest' }],
        }),
      }),
    );
    await page.route('**/api/workloads/web/snapshots', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-fleet').click();
    await expect(page).toHaveURL(/\/fleet/, { timeout: 10_000 });
  });

  test('phase 887: overview unhealthy stat filter', async ({ page }) => {
    await page.route('**/api/orchestrator/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { healthy: 1, degraded: 0, unhealthy: 2, unknown: 0, circuits_open: 0 },
        }),
      }),
    );
    await page.goto('/');
    await page.getByTestId('overview-unhealthy-stat').click();
    await expect(page).toHaveURL(/status=unhealthy/, { timeout: 10_000 });
  });

  test('phase 888: gitops confidential link persists after reload', async ({ page }) => {
    const syncPayload = {
      message: 'Sync completed',
      changes: [],
      confidential_compliance: [
        {
          file_path: 'workloads/web.yaml',
          workload: 'web',
          confidential_enabled: true,
          gitops_issues: [],
          sovereign_compliant: true,
          sovereign_violations: [],
        },
      ],
    };
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { configured: true, repo_url: 'https://github.com/example/repo.git', branch: 'main' },
        }),
      }),
    );
    await page.route('**/api/gitops/preview', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { changes: [] } }),
      }),
    );
    await page.route('**/api/gitops/sync', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: syncPayload }),
      }),
    );
    await page.goto('/gitops');
    await page.getByRole('button', { name: 'Sync now' }).click();
    await page.getByTestId('gitops-sync-confirm-button').click();
    await expect(page.getByTestId('gitops-confidential-link')).toBeVisible({ timeout: 10_000 });
    await page.reload();
    await expect(page.getByTestId('gitops-confidential-link')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 889: compose validate submit testid', async ({ page }) => {
    await page.goto('/compose');
    await expect(page.getByTestId('compose-validate-submit')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 890: alerts events link', async ({ page }) => {
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
    await page.goto('/alerts');
    await page.getByTestId('alerts-events-link').first().click();
    await expect(page).toHaveURL(/\/events/, { timeout: 10_000 });
  });

  test('phase 891: palette sla events deep link', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('sla events');
    await page.getByRole('button', { name: 'Open SLA events feed' }).click();
    await expect(page).toHaveURL(/category=sla/, { timeout: 10_000 });
  });

  test('phase 892: palette intent debugger deep link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('intent debugger');
    await page.getByRole('button', { name: 'Open AI intent debugger' }).click();
    await expect(page).toHaveURL(/\/ai/, { timeout: 10_000 });
    await expect(page).toHaveURL(/tab=analyze/, { timeout: 10_000 });
  });

  test('phase 893: palette templates configure deep link', async ({ page }) => {
    await page.route('**/api/templates', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('templates configure');
    await page.getByRole('button', { name: 'Configure workload template' }).click();
    await expect(page).toHaveURL(/\/templates/, { timeout: 10_000 });
    await expect(page.getByTestId('template-configure-modal')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 894: clusters cluster select URL sync', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            enabled: true,
            cluster_count: 2,
            healthy_clusters: 2,
            clusters: [
              { name: 'dev', reachable: true },
              { name: 'prod', reachable: true },
            ],
          },
        }),
      }),
    );
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { role: 'admin', username: 'admin' } }) }),
    );
    await page.route('**/api/cluster/namespaces**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/cluster/browse**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/cluster/metrics/summary**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.route('**/api/cluster/cilium/status**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.goto('/clusters');
    await page.getByTestId('clusters-cluster-select').selectOption('prod');
    await expect(page).toHaveURL(/cluster=prod/, { timeout: 10_000 });
  });

  test('phase 895: clusters namespace select URL sync', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            enabled: true,
            cluster_count: 1,
            healthy_clusters: 1,
            clusters: [{ name: 'dev', reachable: true }],
          },
        }),
      }),
    );
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: { role: 'admin', username: 'admin' } }) }),
    );
    await page.route('**/api/cluster/namespaces**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'aether-system', workload_count: 1 }],
        }),
      }),
    );
    await page.route('**/api/cluster/browse**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/cluster/metrics/summary**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.route('**/api/cluster/cilium/status**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.goto('/clusters');
    await page.getByTestId('clusters-namespace-select').selectOption('aether-system');
    await expect(page).toHaveURL(/namespace=aether-system/, { timeout: 10_000 });
  });

  test('phase 896: fleet scheduler link', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { enabled: true, cluster_count: 1, healthy_clusters: 1, clusters: [{ name: 'dev', reachable: true }] },
        }),
      }),
    );
    await page.route('**/api/server', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: {} }) }),
    );
    await page.goto('/fleet');
    await page.getByTestId('fleet-scheduler-link').click();
    await expect(page).toHaveURL(/\/scheduler/, { timeout: 10_000 });
  });

  test('phase 897: deps envs link', async ({ page }) => {
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
            stats: { total_workloads: 0, total_edges: 0, root_workloads: 0, leaf_workloads: 0, max_depth: 0, has_cycles: false },
            issues: [],
          },
        }),
      }),
    );
    await page.goto('/deps');
    await page.getByTestId('deps-envs-link').click();
    await expect(page).toHaveURL(/\/envs/, { timeout: 10_000 });
  });

  test('phase 898: overview intent violations quick link', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-intent-violations-quick-link').click();
    await expect(page).toHaveURL(/category=intent-violation/, { timeout: 10_000 });
  });
});
