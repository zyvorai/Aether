// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1181–1230 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });



  test('phase 1183: events workload filter links to scoped alerts', async ({ page }) => {
    await page.route('**/api/events/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total_events: 1, unacknowledged: 0, critical_unacked: 0 },
        }),
      }),
    );
    await page.route('**/api/events**', (route) => {
      if (route.request().url().includes('/summary')) return route.continue();
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [] }),
      });
    });
    await page.goto('/events?workload=web');
    await page.getByTestId('events-alerts-link').first().click();
    await expect(page).toHaveURL(/\/alerts\?workload=web/, { timeout: 10_000 });
  });




});
