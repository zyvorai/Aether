// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect, type Page } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

function statCardValue(page: Page, title: string) {
  return page
    .locator('p.uppercase')
    .filter({ hasText: new RegExp(`^${title}$`, 'i') })
    .locator('xpath=..')
    .locator('p.text-3xl');
}

/**
 * Live UX checks against a deployed Aether (set AETHER_E2E_BASE_URL + AETHER_E2E_SKIP_SERVER=1).
 * Validates dashboard text matches API data — no route mocking.
 */
test.describe('Remote live UX (API-backed)', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('Workloads page stat cards match /api/workloads counts', async ({ page, request }) => {
    const res = await request.get('/api/workloads');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const workloads = body.data ?? body;
    expect(Array.isArray(workloads)).toBeTruthy();
    const total = workloads.length;
    const discovered = workloads.filter((w: { source?: string }) => w.source === 'cluster').length;
    const aether = workloads.filter((w: { source?: string }) => w.source === 'aether').length;

    await page.goto('/workloads');
    await expect(page.getByRole('heading', { name: 'Workloads' })).toBeVisible({ timeout: 15_000 });

    await expect(statCardValue(page, 'Total')).toHaveText(String(total));
    await expect(statCardValue(page, 'Discovered')).toHaveText(String(discovered));
    await expect(statCardValue(page, 'Aether')).toHaveText(String(aether));

    if (discovered > 0) {
      await expect(page.locator('table tbody tr').first()).toBeVisible({ timeout: 10_000 });
    }
  });

  test('Clusters page shows connected cluster from summary API', async ({ page, request }) => {
    const res = await request.get('/api/cluster/summary');
    expect(res.ok()).toBeTruthy();
    const summary = (await res.json()).data;
    expect(summary?.connected).toBeTruthy();

    await page.goto('/clusters');
    await expect(page.getByText('Reachable').first()).toBeVisible({ timeout: 15_000 });
    if (summary.clusters?.[0]?.name) {
      await expect(page.getByText(summary.clusters[0].name).first()).toBeVisible();
    }
  });

  test('Platform page shows Cilium card when API reports cilium CNI', async ({ page, request }) => {
    const res = await request.get('/api/cluster/cilium/status');
    expect(res.ok()).toBeTruthy();
    const cilium = (await res.json()).data;
    expect(cilium?.cni).toBe('cilium');

    await page.goto('/platform');
    await expect(page.getByRole('heading', { name: 'Kubernetes / Cilium' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('allow-aether-egress', { exact: true }).first()).toBeVisible();
    await expect(page.getByText(cilium.egress_mode, { exact: true }).first()).toBeVisible();
    if (cilium.cilium_daemonset_ready) {
      await expect(page.getByText('ready', { exact: true }).first()).toBeVisible();
    }
    if (cilium.metrics_server) {
      await expect(page.getByText('available', { exact: true }).first()).toBeVisible();
    }
  });

  test('Metrics page shows observability summary fields', async ({ page, request }) => {
    const res = await request.get('/api/observability/summary');
    expect(res.ok()).toBeTruthy();
    const obs = (await res.json()).data;
    expect(obs).toHaveProperty('prometheus_configured');

    await page.goto('/metrics');
    await expect(page.getByText('API requests')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Cluster & Cilium')).toBeVisible();
    if (obs.cilium?.cni === 'cilium') {
      await expect(page.getByText('cilium', { exact: false }).first()).toBeVisible();
    }
  });

  test('Clusters Network tab lists Cilium policies from browse API', async ({ page, request }) => {
    const browse = await request.get(
      '/api/cluster/browse?cluster=active-client&namespace=aether-system&kind=CiliumNetworkPolicy',
    );
    expect(browse.ok()).toBeTruthy();
    const items = (await browse.json()).data ?? [];
    test.skip(items.length === 0, 'no CNP resources in cluster');

    await page.goto('/clusters?tab=network');
    await expect(page.getByRole('button', { name: 'Network' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('button', { name: items[0].name }).first()).toBeVisible({ timeout: 15_000 });
  });
});
