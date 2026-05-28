// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1131–1180 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1131: alerts API workload field filters rules', async ({ page }) => {
    await page.route('**/api/alerts/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            channels: [],
            rules: [
              {
                name: 'web-sla',
                enabled: true,
                condition: 'SLA uptime < 99.00%',
                severity: 'Critical',
                message_template: 'web SLA breach',
                cooldown_seconds: 300,
                last_triggered: null,
                workload: 'web',
              },
              {
                name: 'fleet-drift',
                enabled: true,
                condition: 'Drift detected',
                severity: 'Warning',
                message_template: 'Drift detected',
                cooldown_seconds: 3600,
                last_triggered: null,
                workload: null,
              },
            ],
          },
        }),
      }),
    );
    await page.route('**/api/webhooks/queue', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/alerts?workload=web');
    await expect(page.getByTestId('alerts-workload-context')).toContainText('web', { timeout: 10_000 });
    await expect(page.getByText('web-sla')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText('fleet-drift')).toHaveCount(0);
  });

  test('phase 1132: workload detail trust tab for non-kubevirt', async ({ page }) => {
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
    await page.route('**/api/confidential/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.goto('/workloads?workload=web&tab=trust');
    await expect(page.getByRole('button', { name: 'Trust', exact: true })).toBeVisible({ timeout: 10_000 });
    await expect(
      page.getByText(/not configured for confidential computing|GuestKit|Attestation/i).first(),
    ).toBeVisible({
      timeout: 10_000,
    });
  });

  test('phase 1133: confidential fleet attestation badges', async ({ page }) => {
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
    await page.route('**/api/confidential/fleet', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              workload: 'vm-web',
              runtime: 'kubevirt',
              tee: 'sev-snp',
              image_in_catalog: true,
              attestation_passed: true,
              trust: {
                workload: 'vm-web',
                composite: 0.9,
                attestation_score: 0.95,
                network_policy_score: 0.8,
                firmware_exposure_score: 0.9,
              },
            },
          ],
        }),
      }),
    );
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
    await expect(page.getByTestId('confidential-ragnarok-attestation-summary')).toContainText('1/1', { timeout: 10_000 });
    await expect(page.getByTestId('confidential-attestation-badge').first()).toContainText('attested');
  });

  test('phase 1134: guestkit history empty state on trust tab', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'vm-web', status: 'running', runtime: 'kubevirt', image: 'vm:web', source: 'aether' }],
        }),
      }),
    );
    await page.route('**/api/workloads/vm-web/snapshots', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/confidential/workload/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            workload: 'vm-web',
            runtime: 'kubevirt',
            tee: 'sev-snp',
            image_in_catalog: true,
            attestation_passed: false,
            trust: {
              workload: 'vm-web',
              composite: 0.7,
              attestation_score: 0.6,
              network_policy_score: 0.7,
              firmware_exposure_score: 0.8,
            },
          },
        }),
      }),
    );
    await page.route('**/api/confidential/trust-score/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            workload: 'vm-web',
            composite: 0.7,
            attestation_score: 0.6,
            network_policy_score: 0.7,
            firmware_exposure_score: 0.8,
          },
        }),
      }),
    );
    await page.route('**/api/confidential/attestation/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { vm_id: 'vm-web', last_verdict: 'pending', drift: false, updated_at: '2026-01-01T00:00:00Z' },
        }),
      }),
    );
    await page.route('**/api/confidential/secrets/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/confidential/isolation/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { compliant: true, violations: [], scheduler_hints: {} } }),
      }),
    );
    await page.route('**/api/confidential/placement/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            confidential_enabled: true,
            recommended_runtime: 'kubevirt',
            host_tee_ready: true,
            blockers: [],
            gitops_issues: [],
            schedule_constraints: [],
          },
        }),
      }),
    );
    await page.route('**/api/confidential/guestkit/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.route('**/api/confidential/network/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.route('**/api/confidential/intelligence/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.route('**/api/confidential/migration-plan/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.route('**/api/confidential/migration/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.goto('/workloads?workload=vm-web&tab=trust');
    await expect(page.getByTestId('guestkit-history-empty')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('trust-attestation-badge')).toContainText('pending');
  });
});
