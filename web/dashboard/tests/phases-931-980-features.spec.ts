// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

const workloadRow = {
  name: 'web',
  status: 'running',
  runtime: 'kube',
  image: 'nginx:latest',
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

async function mockClusterBrowser(page: import('@playwright/test').Page) {
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
}

test.describe('Phases 931–980 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 931: secrets search URL sync from workload filter', async ({ page }) => {
    await page.route('**/api/secrets', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/secrets?q=web');
    await expect(page.getByPlaceholder('Search secrets…')).toHaveValue('web', { timeout: 10_000 });
    await expect(page.getByTestId('secrets-workload-context')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 932: drift workload context and auto scan', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [workloadRow] }),
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
            has_drift: false,
            severity: 'low',
            drifts: [],
            reconciliation_plan: [],
          },
        }),
      }),
    );
    await page.goto('/drift?workload=web');
    await expect(page.getByTestId('drift-workload-context')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('drift-workload-highlight')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('drift-result-panel')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 933: AI workload context banner', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [workloadRow] }),
      }),
    );
    await page.goto('/ai?workload=web');
    await expect(page.getByTestId('ai-workload-context')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 934: workload detail secrets link scoped', async ({ page }) => {
    await mockWorkloadDetail(page);
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-secrets').click();
    await expect(page).toHaveURL(/\/secrets\?q=web/, { timeout: 10_000 });
  });

  test('phase 935: workload detail drift link opens drift page', async ({ page }) => {
    await mockWorkloadDetail(page);
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-drift').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 936: workload detail events link opens events page', async ({ page }) => {
    await mockWorkloadDetail(page);
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-events').click();
    await expect(page).toHaveURL(/\/events\?workload=web/, { timeout: 10_000 });
  });

  test('phase 937: deploy modal workloads-deploy-submit testid', async ({ page }) => {
    await page.goto('/workloads?deploy=1&source=aether');
    const dialog = page.getByRole('dialog', { name: 'Deploy New Workload' });
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await expect(dialog.getByTestId('workloads-deploy-submit')).toBeVisible({ timeout: 10_000 });
  });

  test('phase 938: drift page events link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/drift');
    await page.getByTestId('drift-events-link').click();
    await expect(page).toHaveURL(/category=drift/, { timeout: 10_000 });
  });

  test('phase 939: palette audit failures deep link', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('audit failures');
    await page.getByRole('button', { name: 'Open audit failures' }).click();
    await expect(page).toHaveURL(/result=failure/, { timeout: 10_000 });
  });

  test('phase 940: palette clusters network tab deep link', async ({ page }) => {
    await mockClusterBrowser(page);
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('cluster network');
    await page.getByRole('button', { name: 'Browse cluster network policies' }).click();
    await expect(page).toHaveURL(/tab=network/, { timeout: 10_000 });
  });

  test('phase 941: palette clusters pods deep link', async ({ page }) => {
    await mockClusterBrowser(page);
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('clusters pods');
    await page.getByRole('button', { name: 'Browse cluster pods' }).click();
    await expect(page).toHaveURL(/kind=Pod/, { timeout: 10_000 });
  });

  test('phase 942: palette compose validate deep link', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible({ timeout: 10_000 });
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('compose validate');
    await page.getByRole('button', { name: 'Validate compose stack' }).click();
    await expect(page).toHaveURL(/\/compose/, { timeout: 10_000 });
  });

  test('phase 943: clusters network tab URL sync', async ({ page }) => {
    await mockClusterBrowser(page);
    await page.goto('/clusters');
    await page.getByTestId('clusters-page-tabs').getByRole('button', { name: 'Network' }).click();
    await expect(page).toHaveURL(/tab=network/, { timeout: 10_000 });
  });

  test('phase 944: fleet drift link', async ({ page }) => {
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
    await page.getByTestId('fleet-drift-link').click();
    await expect(page).toHaveURL(/\/drift/, { timeout: 10_000 });
  });

  test('phase 945: overview audit trail quick link testid', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-audit-quick-link').click();
    await expect(page).toHaveURL(/\/audit/, { timeout: 10_000 });
  });

  test('phase 946: overview scheduler quick link testid', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-scheduler-quick-link').click();
    await expect(page).toHaveURL(/\/scheduler/, { timeout: 10_000 });
  });

  test('phase 947: workload detail backups link scoped', async ({ page }) => {
    await mockWorkloadDetail(page);
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-backups').click();
    await expect(page).toHaveURL(/\/backups\?q=web/, { timeout: 10_000 });
  });

  test('phase 948: overview discovered workloads quick link', async ({ page }) => {
    await page.goto('/');
    await page.getByTestId('overview-discovered-quick-link').click();
    await expect(page).toHaveURL(/source=cluster/, { timeout: 10_000 });
  });
});
