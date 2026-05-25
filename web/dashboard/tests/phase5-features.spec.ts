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
    await page.getByRole('button', { name: 'Intelligence' }).hover();
    await expect(page.getByRole('button', { name: 'Policy Check' })).toHaveCount(0);
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
});
