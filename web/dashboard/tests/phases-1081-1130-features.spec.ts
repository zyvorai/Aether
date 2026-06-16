// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1081–1130 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1081: palette copilot health query deep link', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('fleet health');
    await page.getByRole('button', { name: /Ask copilot about fleet health/i }).click();
    await expect(page).toHaveURL(/\/copilot\?q=/, { timeout: 10_000 });
  });

  test('phase 1082: palette workload copilot deep link', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx', source: 'cluster' }],
        }),
      }),
    );
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('copilot web');
    await page.getByRole('button', { name: 'Ask copilot about: web' }).click();
    await expect(page).toHaveURL(/\/copilot\?q=.*web/, { timeout: 10_000 });
  });

  test('phase 1083: workload detail copilot link prefills query', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'web', status: 'running', runtime: 'kube', image: 'nginx', source: 'cluster' }],
        }),
      }),
    );
    await page.route('**/api/workloads/web/snapshots', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/workloads?workload=web');
    await page.getByTestId('workload-link-copilot').click();
    await expect(page).toHaveURL(/\/copilot\?q=/, { timeout: 10_000 });
    await expect(page.getByTestId('copilot-input')).toHaveValue(/web/i, { timeout: 10_000 });
  });

  test('phase 1084: confidential composite banner and ragnarok link', async ({ page }) => {
    const okList = { success: true, data: [] };
    await page.route('**/api/confidential/capabilities', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            host: { sev_device: true, sev_snp: true, tdx: false, notes: [] },
            clusters: [],
            integration: { mode: 'composite', remote_url: 'https://ragnarok.example/api' },
          },
        }),
      }),
    );
    await page.route('**/api/confidential/fleet', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }));
    await page.route('**/api/confidential/trust-score', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }));
    await page.route('**/api/confidential/sovereign/status', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: {} }) }));
    await page.route('**/api/confidential/images', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }));
    await page.route('**/api/confidential/kata/status', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }));
    await page.route('**/api/confidential/intelligence', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { workloads: [], fleet_trust_avg: 0, critical_count: 0 } }),
      }),
    );
    await page.goto('/confidential');
    await expect(page.getByTestId('confidential-composite-banner')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('confidential-ragnarok-link')).toHaveAttribute('href', 'https://ragnarok.example');
  });

  test('phase 1085: confidential workload intelligence link', async ({ page }) => {
    const okList = { success: true, data: [] };
    await page.route('**/api/confidential/capabilities', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            host: { sev_device: false, sev_snp: false, tdx: false, notes: [] },
            clusters: [],
            integration: { mode: 'embedded' },
          },
        }),
      }),
    );
    await page.route('**/api/confidential/fleet', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }));
    await page.route('**/api/confidential/trust-score', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              workload: 'web',
              composite: 0.82,
              attestation_score: 0.9,
              network_policy_score: 0.75,
              firmware_exposure_score: 0.8,
            },
          ],
        }),
      }),
    );
    await page.route('**/api/confidential/sovereign/status', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: {} }) }));
    await page.route('**/api/confidential/images', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(okList) }));
    await page.route('**/api/confidential/kata/status', (route) => route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }));
    await page.route('**/api/confidential/intelligence', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { workloads: [], fleet_trust_avg: 0, critical_count: 0 } }),
      }),
    );
    await page.goto('/confidential?workload=web');
    await expect(page.getByTestId('confidential-workload-context')).toContainText('web', { timeout: 10_000 });
    await expect(page.getByTestId('confidential-workload-highlight')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('confidential-intelligence-link').click();
    await expect(page).toHaveURL(/\/intelligence\?.*workload=web/, { timeout: 10_000 });
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
