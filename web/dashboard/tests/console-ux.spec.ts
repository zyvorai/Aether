// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * Console shell behaviour: navigation, keyboard use, side panel, triage and
 * failure states. Every API response used here is mocked in the test.
 */

import { test, expect, type Page } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

const workload = (name: string, status: string) => ({
  name,
  status,
  runtime: 'podman',
  image: 'registry.example/app:1',
  created_at: '2026-01-02T03:04:05Z',
  source: 'aether',
});

const WORKLOADS = [workload('alpha', 'running'), workload('bravo', 'failed'), workload('charlie', 'running')];

async function mock(page: Page, path: string, data: unknown) {
  await page.route(`**/api${path}`, (route) =>
    route.fulfill({ contentType: 'application/json', body: JSON.stringify({ success: true, data }) }),
  );
}

async function fail(page: Page, path: string) {
  await page.route(`**/api${path}`, (route) =>
    route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ success: false, error: 'unavailable' }) }),
  );
}

test.describe('Console UX', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
    await page.evaluate(() => {
      localStorage.removeItem('aether-ui-preferences');
      localStorage.removeItem('aether-sidebar-open-sections');
    });
    await mock(page, '/workloads', WORKLOADS);
    await mock(page, '/events', []);
    await mock(page, '/secrets', []);
    await mock(page, '/orchestrator/summary', { healthy: 2, degraded: 0, unhealthy: 1, unknown: 0, circuits_open: 0 });
  });

  test('sidebar finds a nested page and opens it with Enter', async ({ page }) => {
    await page.goto('/');
    const filter = page.getByRole('searchbox', { name: 'Filter navigation' }).first();
    await filter.fill('sbom');
    await filter.press('Enter');
    await expect(page).toHaveURL(/\/security\/sbom/);
    await expect(page).toHaveTitle(/SBOM.*Security Center.*Aether/);
    await expect(page.getByTestId('section-tabs').first().getByRole('link', { name: 'SBOM & images' })).toHaveAttribute('aria-current', 'page');
  });

  test('pinning a page keeps it at the top across reloads', async ({ page }) => {
    await page.goto('/audit');
    await page.getByRole('button', { name: 'Pin this page' }).click();
    await page.reload();
    await expect(page.getByRole('button', { name: 'Unpin this page' })).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByText('Pinned', { exact: true })).toBeVisible();
    await page.locator('#main-content').focus();
    await page.keyboard.press('p');
    await expect(page.getByRole('button', { name: 'Pin this page' })).toBeVisible();
  });

  test('bracket collapses the sidebar and density applies to the document', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByRole('button', { name: 'Collapse navigation' })).toBeVisible();
    await page.locator('#main-content').focus();
    await page.keyboard.press('[');
    await expect(page.getByRole('button', { name: 'Expand navigation' })).toBeVisible();
    await page.getByTestId('theme-toggle').click();
    await page.getByRole('menuitemradio', { name: 'Compact' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-density', 'compact');
  });

  test('workload table opens a side panel from the keyboard and steps through it', async ({ page }) => {
    await page.goto('/workloads');
    const rows = page.getByRole('table', { name: 'Workloads' }).getByRole('rowgroup').getByRole('row');
    await expect(rows).toHaveCount(3);
    await rows.first().focus();
    const second = await rows.nth(1).innerText();
    await page.keyboard.press('j');
    await expect(rows.nth(1)).toBeFocused();
    await page.keyboard.press('Enter');
    const panel = page.getByRole('dialog', { name: /^Workload / });
    await expect(panel).toBeVisible();
    await expect(panel).toContainText('2 of 3');
    expect(second).toBeTruthy();
    await page.keyboard.press('j');
    await expect(panel).toContainText('3 of 3');
    await expect(page).toHaveURL(/workload=/);
    await page.keyboard.press('Escape');
    await expect(panel).toHaveCount(0);
    await expect(page).not.toHaveURL(/workload=/);
  });

  test('sortable column headers report their direction', async ({ page }) => {
    await page.goto('/workloads');
    const header = page.getByRole('columnheader', { name: 'Name' });
    await expect(header).toHaveAttribute('aria-sort', 'none');
    await header.getByRole('button').click();
    await expect(header).toHaveAttribute('aria-sort', 'ascending');
    await header.getByRole('button').click();
    await expect(header).toHaveAttribute('aria-sort', 'descending');
  });

  test('slash focuses the page search', async ({ page }) => {
    await page.goto('/workloads');
    await expect(page.getByTestId('workloads-search')).toBeVisible();
    await page.locator('#main-content').focus();
    await page.keyboard.press('/');
    await expect(page.getByTestId('workloads-search')).toBeFocused();
    await expect(page.getByTestId('workloads-search')).toHaveValue('');
  });

  test('command palette searches by words, runs an action and restores focus', async ({ page }) => {
    await page.goto('/');
    const search = page.getByRole('button', { name: 'Search pages, workloads and actions' });
    await search.click();
    const input = page.getByTestId('command-palette-input');
    await input.fill('logs bravo');
    await expect(page.getByRole('button', { name: /View logs: bravo/ })).toHaveAttribute('aria-current', 'true');
    await input.fill('> compact');
    await input.press('Enter');
    await expect(page.locator('html')).toHaveAttribute('data-density', 'compact');
    await search.click();
    await input.press('Escape');
    await expect(search).toBeFocused();
  });

  test('overview lists what needs attention and links to it', async ({ page }) => {
    await page.goto('/');
    const panel = page.getByTestId('overview-attention');
    await expect(panel.getByRole('heading', { name: '1 thing needs attention' })).toBeVisible();
    await panel.getByRole('link', { name: /bravo is failed/ }).click();
    await expect(page).toHaveURL(/\/workloads\?workload=bravo/);
  });

  test('overview never reports a failed source as healthy', async ({ page }) => {
    await mock(page, '/workloads', [workload('alpha', 'running')]);
    await fail(page, '/events');
    await page.goto('/');
    const panel = page.getByTestId('overview-attention');
    await expect(panel.getByRole('heading', { name: 'Status is incomplete' })).toBeVisible();
    await expect(panel).toContainText('Could not load events');
    await expect(panel).not.toContainText('Nothing needs attention');
  });

  test('a failed events request shows the failing call, not an empty list', async ({ page }) => {
    await fail(page, '/events');
    await page.goto('/events');
    await expect(page.getByRole('alert').filter({ hasText: 'Events unavailable' })).toContainText('GET /api/events: HTTP 503, unavailable');
    await expect(page.getByText('No events have been recorded')).toHaveCount(0);
  });

  test('pausing live updates is visible and reversible', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: /^(Live updates|Reconnecting)/ }).click();
    await page.getByRole('button', { name: 'Pause updates' }).click();
    await expect(page.getByRole('button', { name: /^Automatic updates paused/ })).toBeVisible();
    await page.getByRole('button', { name: 'Resume updates' }).click();
    await expect(page.getByRole('button', { name: /^(Live updates|Reconnecting)/ })).toBeVisible();
  });

  test('phone width: no sideways scroll and the navigation drawer works', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    for (const path of ['/', '/workloads', '/events', '/security', '/settings']) {
      await page.goto(path);
      await expect(page.getByTestId('global-nav')).toBeVisible();
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
      expect(overflow, path).toBe(false);
    }
    await page.getByRole('button', { name: 'Open navigation' }).click();
    const drawer = page.getByRole('dialog', { name: 'Navigation' });
    await expect(drawer).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(drawer).toHaveCount(0);
  });
});
