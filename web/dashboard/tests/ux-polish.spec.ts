import { test, expect, type Page } from '@playwright/test';

async function ensureAuthenticated(page: Page) {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');

  const helpMenu = page.getByRole('button', { name: 'Help menu' });
  if (await helpMenu.isVisible({ timeout: 5000 }).catch(() => false)) {
    return;
  }

  const continueBtn = page.getByRole('button', { name: /continue to dashboard/i });
  if (await continueBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
    await continueBtn.click();
  }

  await expect(helpMenu).toBeVisible({ timeout: 20_000 });
}

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
});
