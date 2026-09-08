// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('Fleet edge and federation tabs', () => {
  test.beforeEach(async ({ page }) => {
    await page.route('**/api/fleet/edge/agents', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              site: 'edge-lab',
              kube_context: 'kind-lab',
              labels: {},
              registered_at: '2026-01-01T00:00:00Z',
              last_heartbeat: new Date().toISOString(),
              queue_depth: 2,
              last_error: null,
              online: true,
            },
          ],
          error: null,
        }),
      });
    });
    await page.route('**/api/fleet/federation/policies', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { clusters: ['kind-lab'], weights: { 'kind-lab': 2 } },
          error: null,
        }),
      });
    });
    await page.route('**/api/fleet/federation/plan', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            workload: 'demo',
            recommended_runtime: 'Kubernetes',
            clusters: [{ cluster: 'kind-lab', score: 80, reachable: true, runtime_hint: 'Kubernetes', reasons: ['reachable'] }],
          },
          error: null,
        }),
      });
    });
    await page.route('**/api/ecosystem/packetwolf/status', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { configured: false, reachable: false }, error: null }),
      });
    });
  });

  test('edge sites tab shows agent health', async ({ page }) => {
    await page.goto('/fleet?tab=edge');
    await expect(page.getByTestId('fleet-edge-tab')).toBeVisible();
    await expect(page.getByText('edge-lab')).toBeVisible();
    await expect(page.getByText(/queue depth/i)).toBeVisible();
  });

  test('placement tab shows federation plan', async ({ page }) => {
    await page.goto('/fleet?tab=placement&workload=demo');
    await expect(page.getByTestId('fleet-placement-tab')).toBeVisible();
    await page.getByTestId('fleet-placement-run').click();
    await expect(page.getByText('kind-lab')).toBeVisible();
  });
});
