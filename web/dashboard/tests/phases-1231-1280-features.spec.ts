// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1231–1280 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });







  test('phase 1234: scheduler workload context links to alerts', async ({ page }) => {
    await page.route('**/api/scheduler/utilization', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/scheduler/suggestions', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/scheduler?workload=web');
    await expect(page.getByTestId('scheduler-workload-context')).toContainText('web');
    await page.getByTestId('scheduler-alerts-link').click();
    await expect(page).toHaveURL(/\/alerts\?workload=web/, { timeout: 10_000 });
  });
});
