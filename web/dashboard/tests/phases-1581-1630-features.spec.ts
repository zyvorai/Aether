// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';


test.describe('Phases 1581–1630 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });



  test('phase 1369: backups workload alias filter', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/backups?workload=web');
    await expect(page.getByTestId('backups-workload-context')).toContainText('web', { timeout: 10_000 });
  });
});
