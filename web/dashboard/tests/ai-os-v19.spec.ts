// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v19 — Live Labs & Reference Cluster', () => {
  test('livelabs overview API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/overview');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.feature_count).toBe(10);
    expect(data.era).toBe('N');
  });

  test('livelabs reference runner API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/reference-runner');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
    expect(data.make_target).toBe('reference-cluster-live');
  });

  test('livelabs kind fixture API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/kind-fixture');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.pod_name).toBe('nginx-playwright');
  });

  test('livelabs live smoke API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/live-smoke');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.steps.length).toBeGreaterThan(0);
  });

  test('livelabs post-deploy verify API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/post-deploy-verify');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.steps.length).toBeGreaterThan(0);
  });

  test('livelabs kubernetes lab API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/kubernetes-lab');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.specs.length).toBeGreaterThan(0);
  });

  test('livelabs advanced runtime labs API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/advanced-runtime-labs');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.specs.some((s: { name: string }) => s.name === 'kubevirt')).toBeTruthy();
  });

  test('livelabs ci pipeline API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/ci-pipeline');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.jobs.length).toBeGreaterThan(0);
  });

  test('livelabs cluster exec API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/cluster-exec');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.apis.length).toBeGreaterThan(0);
  });

  test('livelabs confidential lab API', async ({ request }) => {
    const res = await request.get('/api/intelligence/livelabs/confidential-lab');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.specs.length).toBeGreaterThan(0);
  });

  test('live labs panel on labs page', async ({ page }) => {
    await page.goto('/labs');
    await expect(page.getByTestId('live-labs-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('live-labs-runner-panel')).toBeVisible();

    const agentDock = page.getByTestId('agent-status-dock');
    if (await agentDock.isVisible()) {
      await agentDock.getByRole('button').first().click();
    }

    const pipelineTab = page.getByTestId('live-labs-panel').getByRole('button', { name: 'Pipeline' });
    await pipelineTab.scrollIntoViewIfNeeded();
    await pipelineTab.click();
    await expect(page.getByTestId('live-labs-pipeline-panel')).toBeVisible();
  });
});
