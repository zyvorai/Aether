// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { openNginxPlaywrightPodTerminal } from './helpers/kindCluster';

const kindE2e = process.env.AETHER_E2E_KIND === '1';

test.describe('Cluster exec + port-forward (kind)', () => {
  test.skip(!kindE2e, 'requires AETHER_E2E_KIND=1 and kind-playwright-fixture.sh');

  test('exec terminal shows command output', async ({ page }) => {
    await openNginxPlaywrightPodTerminal(page);

    await page.getByRole('button', { name: /connect/i }).click();
    await page.getByPlaceholder(/command/i).fill('echo aether-exec-ok');
    await page.keyboard.press('Enter');

    await expect(page.locator('pre').filter({ hasText: 'aether-exec-ok' })).toBeVisible({
      timeout: 20_000,
    });
  });

  test('port-forward reaches nginx and stops cleanly', async ({ page, request }) => {
    await openNginxPlaywrightPodTerminal(page);

    await expect(page.getByTestId('clusters-port-forward-panel')).toBeVisible();
    await expect(page.getByTestId('clusters-port-forward-remote')).toHaveValue('80');

    await page.getByTestId('clusters-port-forward-start').click();
    const active = page.getByTestId('clusters-port-forward-active');
    await expect(active).toBeVisible({ timeout: 20_000 });

    const localUrl = await active.locator('.text-emerald-300').textContent();
    expect(localUrl).toMatch(/^http:\/\/127\.0\.0\.1:\d+$/);

    let ready = false;
    for (let attempt = 0; attempt < 15; attempt += 1) {
      const response = await request.get(localUrl!);
      if (response.ok()) {
        const body = await response.text();
        if (body.includes('Welcome to nginx')) {
          ready = true;
          break;
        }
      }
      await page.waitForTimeout(500);
    }
    expect(ready).toBe(true);

    await page.getByTestId('clusters-port-forward-stop').click();
    await expect(active).toBeHidden({ timeout: 10_000 });
  });
});
