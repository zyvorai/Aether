// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

function mockConfidentialFleet(page: import('@playwright/test').Page) {
  const okList = { success: true, data: [] };
  void page.route('**/api/confidential/capabilities', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        data: {
          host: { sev_device: true, sev_snp: true, tdx: false, notes: [] },
          clusters: [],
          integration: { mode: 'local', remote_url: null },
        },
      }),
    }),
  );
  void page.route('**/api/confidential/fleet', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }),
  );
  void page.route('**/api/confidential/trust-score', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }),
  );
  void page.route('**/api/confidential/sovereign/status', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: {} }) }),
  );
  void page.route('**/api/confidential/images', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }),
  );
  void page.route('**/api/confidential/kata/status', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
  );
  void page.route('**/api/confidential/intelligence', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true, data: { workloads: [], fleet_trust_avg: 0, critical_count: 0 } }),
    }),
  );
}

test.describe('Phases 1581–1630 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1584: confidential workload context drift link', async ({ page }) => {
    mockConfidentialFleet(page);
    await page.goto('/confidential?workload=web');
    await expect(page.getByTestId('confidential-workload-context')).toContainText('web', { timeout: 15_000 });
    await page.getByTestId('confidential-drift-link').click();
    await expect(page).toHaveURL(/\/drift\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1603: confidential workload context metrics link', async ({ page }) => {
    mockConfidentialFleet(page);
    await page.goto('/confidential?workload=web');
    await expect(page.getByTestId('confidential-workload-context')).toContainText('web', { timeout: 15_000 });
    await page.getByTestId('confidential-metrics-link').click();
    await expect(page).toHaveURL(/\/metrics\?workload=web/, { timeout: 10_000 });
  });

  test('phase 1369: backups workload alias filter', async ({ page }) => {
    await page.route('**/api/backups', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/backups?workload=web');
    await expect(page.getByTestId('backups-workload-context')).toContainText('web', { timeout: 10_000 });
  });
});
