// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1381–1430 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1381: deploy success intelligence link', async ({ page }) => {
    const unique = `intel-${Date.now().toString(36)}`;
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
    await dialog.getByTestId('deploy-success-intelligence-link').click();
    await expect(page).toHaveURL(new RegExp(`/intelligence\\?workload=${unique}`), { timeout: 10_000 });
  });

  test('phase 1394: workload events tab health link', async ({ page }) => {
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
    await page.getByTestId('workload-events-health-link').click();
    await expect(page).toHaveURL(/\/health-monitor\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1401: deploy success gitops link', async ({ page }) => {
    const unique = `gitops-${Date.now().toString(36)}`;
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
    await dialog.getByTestId('deploy-success-gitops-link').click();
    await expect(page).toHaveURL(new RegExp(`/gitops\\?workload=${unique}`), { timeout: 10_000 });
  });

  test('phase 1417: compose workload context banner', async ({ page }) => {
    await page.goto('/compose?workload=web');
    await expect(page.getByTestId('compose-workload-context')).toContainText('web', { timeout: 10_000 });
  });
});
