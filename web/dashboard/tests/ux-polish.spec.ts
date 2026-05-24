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
});
