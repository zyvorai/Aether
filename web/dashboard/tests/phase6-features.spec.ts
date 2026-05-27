// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phase 6 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('nav shows setup-required items that link to Platform', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            version: '0.3.0',
            safety: {
              mutation_confirm_required: false,
              mutation_confirm_header: 'X-Aether-Confirm',
              mutation_confirm_values: ['1'],
            },
            opa: { configured: false, enforce: false },
            oidc: { enabled: false },
            workload_state: { backend: 'local-json', configured: true },
          },
        }),
      }),
    );
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { configured: false } }),
      }),
    );
    await page.reload();
    await page.getByRole('button', { name: 'Intelligence' }).click();
    await expect(page.getByText('Setup required')).toBeVisible({ timeout: 10_000 });
    await page.getByRole('menuitem', { name: /Policy Check/i }).click();
    await expect(page).toHaveURL(/\/platform/, { timeout: 10_000 });
  });

  test('editor edit YAML directly enables CodeMirror editing', async ({ page }) => {
    await page.goto('/editor');
    await page.getByRole('button', { name: 'Edit YAML directly' }).click();
    const editor = page.getByRole('textbox', { name: /workload yaml preview/i });
    await editor.click();
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.insertText('name: yaml-edit-test');
    await expect(editor).toContainText('yaml-edit-test');
  });

  test('gitops preview file path links to workload', async ({ page }) => {
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { configured: true, repo_url: 'https://github.com/org/workloads.git', branch: 'main' },
        }),
      }),
    );
    await page.route('**/api/gitops/preview', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            changes: [{ file_path: 'workloads/httpd.yaml', change_type: 'Modified', commit: 'abc123' }],
          },
        }),
      }),
    );

    await page.goto('/gitops');
    await page.getByRole('button', { name: 'Sync now' }).click();
    await page.getByTestId('gitops-diff-preview').getByRole('button', { name: /workloads\/httpd\.yaml/i }).click();
    await expect(page).toHaveURL(/workload=httpd/, { timeout: 10_000 });
  });

  test('compose down button visible after valid compose', async ({ page }) => {
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

    await page.goto('/compose');
    await page.getByRole('button', { name: 'Validate compose' }).click();
    await expect(page.getByRole('button', { name: 'Compose down' })).toBeVisible({ timeout: 10_000 });
  });

  test('workload detail shows snapshots panel for aether workloads', async ({ page }) => {
    await page.route('**/api/workloads', (route) => {
      if (route.request().method() === 'GET') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: [
              {
                name: 'httpd',
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
    await page.route('**/api/workloads/*/snapshots', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ version: 0, path: '/tmp/snap-0.json' }],
        }),
      }),
    );

    await page.goto('/workloads?workload=httpd');
    await expect(page.getByTestId('workload-snapshots')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('workload-snapshots')).toContainText('v0');
  });
});
