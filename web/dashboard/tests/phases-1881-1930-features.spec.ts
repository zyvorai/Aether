// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1881–1930 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1881: compose editor link', async ({ page }) => {
    await page.goto('/compose?workload=web');
    await expect(page.getByTestId('compose-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('compose-editor-link').click();
    await expect(page).toHaveURL(/\/editor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1886: deps compose link', async ({ page }) => {
    await page.goto('/deps?workload=web');
    await expect(page.getByTestId('deps-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('deps-workload-context').getByTestId('deps-context-compose-link').click();
    await expect(page).toHaveURL(/\/compose\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1891: gitops policy link', async ({ page }) => {
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { configured: true, repo_url: '', branch: 'main', last_sync: null } }),
      }),
    );
    await page.goto('/gitops?workload=web');
    await expect(page.getByTestId('gitops-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('gitops-workload-context').getByTestId('gitops-context-policy-link').click();
    await expect(page).toHaveURL(/\/policy\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1896: health copilot link', async ({ page }) => {
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('health-copilot-link').click();
    await expect(page).toHaveURL(/\/copilot\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1906: workloads list openapi link', async ({ page }) => {
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
    await page.getByTestId('workloads-openapi-link').click();
    await expect(page).toHaveURL(/\/openapi\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1911: overview intelligence link', async ({ page }) => {
    await page.goto('/?workload=web');
    await expect(page.getByTestId('overview-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('overview-intelligence-link').click();
    await expect(page).toHaveURL(/\/intelligence\?workload=web.*tab=predictions/, { timeout: 10_000 });
  });

  test('phase 1921: deploy success policy link', async ({ page }) => {
    const unique = `policy-${Date.now().toString(36)}`;
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
    await dialog.getByTestId('deploy-success-policy-link').click();
    await expect(page).toHaveURL(new RegExp(`/policy\\?workload=${unique}`), { timeout: 10_000 });
  });
});
