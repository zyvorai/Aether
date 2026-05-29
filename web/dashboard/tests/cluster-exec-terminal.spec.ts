// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from '@playwright/test';

const kindE2e = process.env.AETHER_E2E_KIND === '1';

test.describe('Cluster exec terminal (kind)', () => {
  test.skip(!kindE2e, 'requires AETHER_E2E_KIND=1 and kind-playwright-fixture.sh');

  test('exec terminal shows command output', async ({ page }) => {
    await page.goto('/clusters');
    await page.getByTestId('cluster-browser').waitFor({ timeout: 30_000 });

    const clusterSelect = page.locator('select').filter({ hasText: /kind-/i }).first();
    if (await clusterSelect.count()) {
      await clusterSelect.selectOption({ index: 0 });
    }

    await page.getByRole('button', { name: /default/i }).first().click({ timeout: 15_000 }).catch(() => {});
    await page.getByText('nginx-playwright').first().click({ timeout: 15_000 });

    await page.getByRole('button', { name: /terminal/i }).click();
    await page.getByRole('button', { name: /connect/i }).click();

    await page.getByPlaceholder(/command/i).fill('echo aether-exec-ok');
    await page.keyboard.press('Enter');

    await expect(page.locator('pre').filter({ hasText: 'aether-exec-ok' })).toBeVisible({
      timeout: 20_000,
    });
  });
});
