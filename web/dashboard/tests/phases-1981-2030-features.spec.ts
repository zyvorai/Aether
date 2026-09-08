// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1981–2030 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1981: overview policy link', async ({ page }) => {
    await page.goto('/?workload=web');
    await expect(page.getByTestId('overview-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('overview-policy-link').click();
    await expect(page).toHaveURL(/\/policy\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1986: platform intelligence link', async ({ page }) => {
    await page.goto('/platform?workload=web');
    await expect(page.getByTestId('platform-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('platform-links-more').click();
    await page.getByTestId('platform-intelligence-link').click();
    await expect(page).toHaveURL(/\/intelligence\?workload=web.*tab=predictions/, { timeout: 10_000 });
  });

  test('phase 1991: gitops editor link', async ({ page }) => {
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
    await page.getByTestId('gitops-context-editor-link').click();
    await expect(page).toHaveURL(/\/editor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1996: secrets policy link', async ({ page }) => {
    await page.route('**/api/secrets', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/secrets?workload=web');
    await expect(page.getByTestId('secrets-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('secrets-context-policy-link').click();
    await expect(page).toHaveURL(/\/policy\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2006: health policy link', async ({ page }) => {
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('health-context-policy-link').click();
    await expect(page).toHaveURL(/\/policy\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2016: metrics scheduler link', async ({ page }) => {
    await page.route('**/api/metrics/observability**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { prometheus_up: true, grafana_up: false } }),
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
    await page.goto('/metrics?workload=web');
    await expect(page.getByTestId('metrics-workload-context')).toContainText('web', { timeout: 10_000 });
    await page.getByTestId('metrics-context-scheduler-link').click();
    await expect(page).toHaveURL(/\/scheduler\?workload=web/, { timeout: 10_000 });
  });

  test('phase 2026: deploy success backups link', async ({ page }) => {
    const unique = `backup-${Date.now().toString(36)}`;
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
    await dialog.getByTestId('deploy-success-backups-link').click();
    await expect(page).toHaveURL(new RegExp(`/backups\\?workload=${unique}`), { timeout: 10_000 });
  });
});
