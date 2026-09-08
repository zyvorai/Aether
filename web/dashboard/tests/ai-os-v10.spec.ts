// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v10 — Knowledge & Graph', () => {
  test('interactive graph API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/interactive?edge_kinds=depends_on');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.nodes)).toBeTruthy();
  });

  test('impact analysis API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/impact?workload=api');
    expect(res.ok()).toBeTruthy();
  });

  test('blast radius API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/blast-radius?workload=api');
    expect(res.ok()).toBeTruthy();
  });

  test('k8s import dry-run API', async ({ request }) => {
    const res = await request.post('/api/intelligence/graph/import-k8s', { data: { dry_run: true } });
    expect(res.ok()).toBeTruthy();
  });

  test('threat paths API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/threat-paths');
    expect(res.ok()).toBeTruthy();
  });

  test('graph search API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/search?q=workload');
    expect(res.ok()).toBeTruthy();
  });

  test('graph snapshots API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/snapshots');
    expect(res.ok()).toBeTruthy();
  });

  test('cmdb inventory API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/cmdb');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.items.length).toBeGreaterThan(0);
  });

  test('graph placement API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/placement');
    expect(res.ok()).toBeTruthy();
  });

  test('graph export API', async ({ request }) => {
    const res = await request.get('/api/intelligence/graph/export?format=neo4j');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.payload).toBeTruthy();
    expect(String(data.payload).length).toBeGreaterThan(10);
  });

  test('knowledge graph panel with filters', async ({ page }) => {
    await page.route('**/api/intelligence/graph/interactive**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            nodes: [{ id: 'wl:web', label: 'web', kind: 'workload' }],
            edges: [],
            stats: { workloads: 1, dependencies: 0, threats: 0, drifted: 0, clusters: 0 },
          },
        }),
      }),
    );
    await page.goto('/labs');
    await expect(page.getByTestId('knowledge-graph-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('graph-edge-filters')).toBeVisible();
    await expect(page.getByTestId('interactive-graph-canvas')).toBeVisible();
  });

  test('graph platform panel', async ({ page }) => {
    await page.goto('/labs');
    await expect(page.getByTestId('graph-platform-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('graph-impact-panel')).toBeVisible();
  });
});
