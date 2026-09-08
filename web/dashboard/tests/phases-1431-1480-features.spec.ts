// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1431–1480 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });


  test('phase 1454: policy workload context banner', async ({ page }) => {
    await page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { opa: { configured: true } } }),
      }),
    );
    await page.goto('/policy?workload=web');
    await expect(page.getByTestId('policy-workload-context')).toContainText('web', { timeout: 10_000 });
  });

  test('phase 1461: workloads list context banner when detail closed', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'db', status: 'running', runtime: 'kube', image: 'postgres', source: 'aether' }],
        }),
      }),
    );
    await page.goto('/workloads?workload=web');
    await expect(page.getByTestId('workloads-context-banner')).toContainText('web', { timeout: 10_000 });
  });
});
