// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Phases 1181–1230 features', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('phase 1181: workload detail trust quick link opens trust tab', async ({ page }) => {
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
    await page.route('**/api/confidential/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: null }) }),
    );
    await page.goto('/workloads?workload=vm-web');
    await page.getByTestId('workload-link-trust').click();
    await expect(page).toHaveURL(/tab=trust/, { timeout: 10_000 });
    await expect(page.getByTestId('workload-tab-trust')).toBeVisible();
  });

  test('phase 1182: palette opens trust tab for workload', async ({ page }) => {
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
    await page.goto('/');
    await page.keyboard.press('ControlOrMeta+k');
    await page.getByPlaceholder('Search pages, workloads, and actions…').fill('trust vm-web');
    await page.getByRole('button', { name: /Trust tab: vm-web/i }).click();
    await expect(page).toHaveURL(/workload=vm-web.*tab=trust/, { timeout: 10_000 });
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

  test('phase 1184: health detail links to alerts and trust', async ({ page }) => {
    await page.route('**/api/orchestrator/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: { healthy: 1, degraded: 0, unhealthy: 0, unknown: 0, circuits_open: 0 },
        }),
      }),
    );
    await page.route('**/api/orchestrator/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              name: 'web',
              runtime: 'kube',
              health: 'healthy',
              circuit: 'closed',
              restart_count: 0,
            },
          ],
        }),
      }),
    );
    await page.route('**/api/health/web', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            total_checks: 10,
            ready_checks: 10,
            uptime_percent: 99.9,
            last_state: 'ready',
            last_restart_count: 0,
          },
        }),
      }),
    );
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-detail-panel')).toBeVisible({ timeout: 15_000 });
    await page.getByTestId('health-alerts-link').first().click();
    await expect(page).toHaveURL(/\/alerts\?workload=web/, { timeout: 10_000 });
    await page.goto('/health-monitor?workload=web');
    await expect(page.getByTestId('health-detail-panel')).toBeVisible({ timeout: 15_000 });
    await page.getByTestId('health-trust-link').click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1185: confidential workload context cross-links', async ({ page }) => {
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
            integration: { mode: 'local', remote_url: null },
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
    await page.goto('/confidential?workload=vm-web');
    await expect(page.getByTestId('confidential-workload-context')).toContainText('vm-web');
    await page.getByTestId('confidential-trust-link').click();
    await expect(page).toHaveURL(/workload=vm-web.*tab=trust/, { timeout: 10_000 });
    await page.goto('/confidential?workload=vm-web');
    await page.getByTestId('confidential-alerts-link').click();
    await expect(page).toHaveURL(/\/alerts\?workload=vm-web/, { timeout: 10_000 });
  });

  test('phase 1187: alert rule workload chip links to trust tab', async ({ page }) => {
    await page.route('**/api/alerts/status**', (route) =>
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
            ],
          },
        }),
      }),
    );
    await page.route('**/api/webhooks/queue', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, data: [] }) }),
    );
    await page.goto('/alerts?workload=web');
    await page.getByTestId('alerts-rule-workload-web').click();
    await expect(page).toHaveURL(/workload=web.*tab=trust/, { timeout: 10_000 });
  });

  test('phase 1188: attest-gated secrets empty state on confidential workload', async ({ page }) => {
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
    await expect(page.getByTestId('attest-secrets-empty')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('trust-attestation-badge')).toContainText('pending');
  });
});
