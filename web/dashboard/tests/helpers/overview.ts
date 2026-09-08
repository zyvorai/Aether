// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { expect, type Page } from '@playwright/test';

/** Minimal overview API mocks so quick-access chips render under parallel load. */
export async function mockOverviewApis(page: Page) {
  const handlers: Array<[string, unknown]> = [
    ['**/api/workloads', []],
    ['**/api/events/summary', { total_events: 0, unacknowledged: 0, critical_unacked: 0 }],
    ['**/api/orchestrator/summary', { healthy: 0, degraded: 0, unhealthy: 0 }],
    ['**/api/backups', []],
    ['**/api/secrets', []],
    ['**/api/events', []],
    ['**/api/cluster/summary', { enabled: false, connected: false, cluster_count: 0, clusters: [] }],
    ['**/api/plugins', []],
    ['**/api/environments', []],
    ['**/api/rbac/keys', []],
  ];

  for (const [pattern, data] of handlers) {
    await page.route(pattern, (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data }),
      }),
    );
  }
}

/** Wait until overview quick links are interactive (past loading/error gates). */
export async function waitForOverviewReady(page: Page) {
  await mockOverviewApis(page);
  await page.goto('/');
  await expect(page.getByTestId('command-center-briefing')).toBeVisible({ timeout: 25_000 });
  const quick = page.getByTestId('overview-platform-quick-link');
  await quick.scrollIntoViewIfNeeded();
  await expect(quick).toBeVisible({ timeout: 15_000 });
}

/** Click a test id after scrolling it into view (handles duplicate ids via .first()). */
export async function clickTestId(page: Page, id: string) {
  const target = page.getByTestId(id).first();
  await target.scrollIntoViewIfNeeded();
  await target.click();
}

/** Scroll overview quick link into view before visibility/click assertions. */
export async function scrollOverviewQuickLink(page: Page, id: string) {
  const target = page.getByTestId(id).first();
  await target.scrollIntoViewIfNeeded();
  return target;
}
