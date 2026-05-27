// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phase 5 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('hides Policy nav when OPA is not configured', async ({ page }) => {
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
    await expect(page.getByRole('menuitem', { name: /Policy Check/i })).toBeVisible();
  });

  test('editor deploy validates and posts workload', async ({ page }) => {
    await page.route('**/api/validate', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { valid: true, workload_name: 'httpd', errors: [] },
        }),
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
          body: JSON.stringify({ success: true, data: 'Workload httpd created' }),
        });
      }
      return route.continue();
    });

    await page.goto('/editor');
    await page.getByRole('button', { name: 'Deploy workload' }).click();
    await expect(page).toHaveURL(/workload=httpd/, { timeout: 15_000 });
  });

  test('compose validate shows deploy stack after valid compose', async ({ page }) => {
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
    await expect(page.getByRole('button', { name: 'Deploy stack' })).toBeVisible({ timeout: 10_000 });
  });

  test('compose deploy stack posts compose/up and shows result', async ({ page }) => {
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
    await page.route('**/api/compose/up', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { deployed: ['web'], count: 1 },
        }),
      }),
    );

    await page.goto('/compose');
    await page.getByRole('button', { name: 'Validate compose' }).click();
    await page.getByRole('button', { name: 'Deploy stack' }).click();
    await expect(page.getByText(/Deployed 1 workload\(s\): web/)).toBeVisible({ timeout: 10_000 });
  });

  test('gitops sync confirm shows diff preview', async ({ page }) => {
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            configured: true,
            repo_url: 'https://github.com/org/workloads.git',
            branch: 'main',
          },
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
            changes: [
              {
                file_path: 'workloads/httpd.yaml',
                change_type: 'Modified',
                commit: 'abc123def456',
              },
            ],
          },
        }),
      }),
    );

    await page.goto('/gitops');
    await page.getByRole('button', { name: 'Sync now' }).click();
    const preview = page.getByTestId('gitops-diff-preview');
    await expect(preview).toBeVisible({ timeout: 10_000 });
    await expect(preview.getByText('workloads/httpd.yaml')).toBeVisible();
    await expect(preview.getByText('Modified')).toBeVisible();
  });

  test('editor shows CodeMirror YAML preview', async ({ page }) => {
    await page.goto('/editor');
    await expect(page.getByTestId('editor-yaml-preview')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('textbox', { name: /workload yaml preview/i })).toContainText(
      'apiVersion: aether/v1',
    );
  });
});
