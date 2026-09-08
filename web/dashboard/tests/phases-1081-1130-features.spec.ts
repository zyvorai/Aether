// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1081–1130 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });






  test('phase 1086: audit workload context banner', async ({ page }) => {
    await page.route('**/api/audit/verify', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total: 1, verified: 1, tampered: 0, integrity: 'ok', tampered_events: [] },
        }),
      }),
    );
    await page.route('**/api/audit', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            summary: {
              total_events: 1,
              successes: 1,
              failures: 0,
              unique_workloads: 1,
              events_by_action: { deploy: 1 },
            },
            recent_events: [
              {
                id: 1,
                timestamp: '2026-01-01T00:00:00Z',
                action: 'deploy',
                workload: 'web',
                result: 'success',
                runtime: 'kube',
                message: 'ok',
              },
            ],
          },
        }),
      }),
    );
    await page.route('**/api/audit/verify', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { total: 1, verified: 1, tampered: 0, integrity: 'ok', tampered_events: [] },
        }),
      }),
    );
    await page.goto('/audit?workload=web');
    await expect(page.getByTestId('audit-workload-context')).toContainText('web', { timeout: 10_000 });
  });
});
