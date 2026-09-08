// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1731–1780 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1731: secrets editor link from workload filter', async ({ page }) => {
    await page.route('**/api/secrets', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/secrets?workload=web');
    await expect(page.getByTestId('secrets-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('secrets-editor-link').click();
    await expect(page).toHaveURL(/\/editor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1736: backups drift link from workload filter', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/backups?workload=web');
    await expect(page.getByTestId('backups-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('backups-drift-link').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1741: gitops platform cross-link', async ({ page }) => {
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { configured: true, repo_url: '', branch: 'main', last_sync: null } }),
      }),
    );
    await page.goto('/gitops?workload=web');
    await expect(page.getByTestId('gitops-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('gitops-links-more').click();
    await page.getByTestId('gitops-platform-link').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1746: audit openapi link', async ({ page }) => {
    await page.route('**/api/audit/verify', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total: 1, verified: 1, tampered: 0, integrity: 'ok', tampered_events: [] },
        }),
      }),
    );
    await page.route('**/api/audit', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            summary: {
              total_events: 1,
              successes: 1,
              failures: 0,
              unique_workloads: 1,
              events_by_action: { deploy: 1 },
            },
            recent_events: [
              {
                id: 1,
                timestamp: '2026-01-01T00:00:00Z',
                action: 'deploy',
                workload: 'web',
                result: 'success',
                runtime: 'kube',
                message: 'ok',
              },
            ],
          },
        }),
      }),
    );
    await page.goto('/audit?workload=web');
    await expect(page.getByTestId('audit-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('audit-openapi-link').click();
    await expect(page).toHaveURL(/\/openapi\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1751: compose deps link', async ({ page }) => {
    await page.goto('/compose?workload=web');
    await expect(page.getByTestId('compose-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('compose-deps-link').click();
    await expect(page).toHaveURL(/\/deps\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1761: envs editor link', async ({ page }) => {
    await page.route('**/api/environments', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/envs?workload=web');
    await expect(page.getByTestId('envs-workload-banner')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('envs-editor-link').click();
    await expect(page).toHaveURL(/\/editor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1766: workload detail platform quick link', async ({ page }) => {
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
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-platform').click();
    await expect(page).toHaveURL(/\/platform\?workload=web/, { timeout: 10_000 });
  });
});
