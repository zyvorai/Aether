// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

test.describe('AI troubleshoot apply fix', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('shows Apply fix for applyable restart recommendation', async ({ page }) => {
    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [
            {
              name: 'prod/default/api',
              status: 'degraded',
              runtime: 'kube',
              image: 'nginx:latest',
              source: 'cluster',
              cluster: 'kind-aether',
              namespace: 'default',
              kind: 'Deployment',
            },
          ],
        }),
      }),
    );

    await page.route('**/api/copilot/troubleshoot', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            workload: 'prod/default/api',
            source: 'cluster',
            health_level: 'failing',
            summary: '0/2 pods ready',
            ready_pods: 0,
            total_pods: 2,
            warning_events: 1,
            events: [],
            pods: [],
            evidence: ['CrashLoopBackOff'],
            log_excerpt: 'CrashLoopBackOff: back-off restarting failed container',
            recommendations: [
              {
                title: 'CrashLoopBackOff detected',
                summary: 'Restart the workload after fixing config, or restart now to retry.',
                action: 'restart',
                applyable: true,
              },
            ],
          },
        }),
      }),
    );

    let applied = false;
    await page.route('**/api/cluster/action', (route) => {
      applied = true;
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: 'restart Deployment default/api on cluster kind-aether' }),
      });
    });

    await page.goto('/workloads');
    await page.getByText('prod/default/api').click();
    await expect(page.getByTestId('ai-troubleshoot-panel')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('ai-troubleshoot-diagnose').click();
    await expect(page.getByTestId('ai-troubleshoot-apply-restart')).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('ai-troubleshoot-apply-restart').click();
    await expect(page.getByText('Restart Workload')).toBeVisible({ timeout: 10_000 });
    expect(applied).toBe(true);
  });
});
