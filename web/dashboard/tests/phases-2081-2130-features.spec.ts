// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 2081–2130 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 2081: overview compose link', async ({ page }) => {
    await page.goto('/?workload=web');
    await expect(page.getByTestId('overview-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('overview-compose-link').click();
    await expect(page).toHaveURL(/\/compose\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2086: platform editor link', async ({ page }) => {
    await page.goto('/platform?workload=web');
    await expect(page.getByTestId('platform-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('platform-links-more').click();
    await page.getByTestId('platform-editor-link').click();
    await expect(page).toHaveURL(/\/editor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2091: health secrets link', async ({ page }) => {
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('health-context-secrets-link').click();
    await expect(page).toHaveURL(/\/secrets\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2096: compose gitops link', async ({ page }) => {
    await page.goto('/compose?workload=web');
    await expect(page.getByTestId('compose-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('compose-workload-context').getByTestId('compose-context-gitops-link').click();
    await expect(page).toHaveURL(/\/gitops\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2101: workloads gitops link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'api', status: 'running', runtime: 'kube', image: 'nginx', source: 'cluster' }],
        }),
      }),
    );
    await page.goto('/workloads?workload=web');
    await expect(page.getByTestId('workloads-context-banner')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('workloads-links-more').click();
    await page.getByTestId('workloads-context-gitops-link').click();
    await expect(page).toHaveURL(/\/gitops\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2116: drift openapi link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx' }],
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
            has_drift: false,
            severity: 'none',
            drifts: [],
            reconciliation_plan: [],
          },
        }),
      }),
    );
    await page.goto('/drift?workload=web');
    await expect(page.getByTestId('drift-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('drift-links-more').click();
    await page.getByTestId('drift-context-openapi-link').click();
    await expect(page).toHaveURL(/\/openapi\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2121: deploy success fleet link', async ({ page }) => {
    const unique = `fleet-${Date.now().toString(36)}`;
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
    await dialog.getByTestId('deploy-success-fleet-link').click();
    await expect(page).toHaveURL(new RegExp(`/fleet\\?workload=${unique}`), { timeout: 10_000 });
  });
});
