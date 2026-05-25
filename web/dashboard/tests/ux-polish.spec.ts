import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Dashboard UX polish', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('Help menu opens shortcuts in Help dialog', async ({ page }) => {
    await page.getByRole('button', { name: 'Help menu' }).click();
    await page.getByRole('menuitem', { name: 'Keyboard shortcuts' }).click();
    await expect(page.getByRole('dialog', { name: 'Help' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Shortcuts' })).toHaveAttribute('aria-selected', 'true');
  });

  test('command palette Help: about opens About tab', async ({ page }) => {
    await page.keyboard.press('Escape');
    const mod = process.platform === 'darwin' ? 'Meta' : 'Control';
    await page.keyboard.press(`${mod}+KeyK`);
    await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible({ timeout: 10_000 });
    const input = page.getByPlaceholder('Search pages, workloads, and actions…');
    await input.fill('help about');
    await page.getByRole('button', { name: 'Help: about Aether' }).click();
    await expect(page.getByRole('dialog', { name: 'Help' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'About' })).toHaveAttribute('aria-selected', 'true');
  });

  test('workloads validate query opens modal', async ({ page }) => {
    await page.goto('/workloads?validate=1');
    await expect(page.getByRole('heading', { name: 'Validate Workload YAML' })).toBeVisible({
      timeout: 15_000,
    });
  });

  test('workloads deploy query opens modal', async ({ page }) => {
    await page.goto('/workloads?deploy=1');
    await expect(page.getByRole('heading', { name: 'Deploy New Workload' })).toBeVisible({
      timeout: 15_000,
    });
  });

  test('GitOps page shows retry UI when status API fails', async ({ page }) => {
    await page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 503,
        contentType: 'application/json',
        body: JSON.stringify({ success: false, error: 'unavailable' }),
      }),
    );
    await page.goto('/gitops');
    await expect(page.getByText('GitOps status unavailable')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
  });

  test('empty overview shows onboarding strip', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Getting started' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/0\/3 steps complete/i)).toBeVisible();
  });

  test('onboarding validate step opens validate modal', async ({ page }) => {
    await page.goto('/');
    const validateStep = page.getByRole('button', { name: /Validate a spec/i });
    await expect(validateStep).toBeVisible({ timeout: 15_000 });
    await validateStep.click();
    await expect(page.getByRole('heading', { name: 'Validate Workload YAML' })).toBeVisible({
      timeout: 15_000,
    });
  });

  test('onboarding deploy step opens deploy modal', async ({ page }) => {
    await page.goto('/');
    const deployStep = page.getByRole('button', { name: /Deploy first workload/i });
    await expect(deployStep).toBeVisible({ timeout: 15_000 });
    await deployStep.click();
    await expect(page.getByRole('heading', { name: 'Deploy New Workload' })).toBeVisible({
      timeout: 15_000,
    });
  });

  test('Policy page shows OPA probe retry when server API fails', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 503,
        contentType: 'application/json',
        body: JSON.stringify({ success: false, error: 'unavailable' }),
      }),
    );
    await page.goto('/policy');
    await expect(page.getByText(/Could not load OPA status/i)).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('button', { name: 'Retry' }).first()).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Workload policy check' })).toBeVisible();
  });

  test('light theme deploy modal opens with readable dialog', async ({ page }) => {
    await page.getByLabel('Theme').selectOption('light');
    await page.goto('/workloads?deploy=1');
    await expect(page.getByRole('heading', { name: 'Deploy New Workload' })).toBeVisible({
      timeout: 15_000,
    });
    const dialog = page.getByRole('dialog', { name: 'Deploy New Workload' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Deploy' })).toBeEnabled();
    await expect(dialog.locator('textarea')).toHaveValue(/apiVersion: aether\/v1/);
  });

  test('editor validate marks onboarding validate step in localStorage', async ({ page }) => {
    await page.route('**/api/validate', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { valid: true, workload_name: 'my-app', errors: [] },
        }),
      }),
    );
    await ensureAuthenticated(page);
    await page.goto('/editor');
    await page.evaluate(() => localStorage.removeItem('aether_onboarding_validated'));
    await page.getByRole('button', { name: 'Validate' }).click();
    await expect
      .poll(async () => page.evaluate(() => localStorage.getItem('aether_onboarding_validated')))
      .toBe('1');
  });

  test('viewer role hides deploy button on workloads page', async ({ page }) => {
    await page.route('**/api/auth/me', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { authenticated: true, username: 'viewer-user', role: 'viewer' },
        }),
      }),
    );
    await ensureAuthenticated(page);
    await page.reload();
    await page.goto('/workloads');
    await expect(page.getByText(/Read-only session/i)).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('button', { name: 'Deploy' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Deploy YAML' })).toHaveCount(0);
  });
});
