// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

test.describe('AI Infrastructure OS v16 — Lab Graduation', () => {
  test('labs overview API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/overview');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.graduated_count).toBe(10);
  });

  test('labs terraform export API', async ({ request }) => {
    const res = await request.post('/api/intelligence/labs/terraform-export', {
      data: { goals: ['cost-optimized'], workload_name: 'demo' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
    expect(data.hcl).toContain('aether_workload');
  });

  test('labs pulumi bridge API', async ({ request }) => {
    const res = await request.post('/api/intelligence/labs/pulumi-bridge', {
      data: { goals: ['ha'], language: 'typescript' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('labs mobile companion API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/mobile-companion');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
    expect(data.pwa_manifest.name).toBe('Aether Fleet');
  });

  test('labs ide extensions API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/ide-extensions');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.install_commands.length).toBeGreaterThan(0);
  });

  test('labs community intents API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/community-intents');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.community.length).toBeGreaterThan(0);
  });

  test('labs community import API', async ({ request }) => {
    const res = await request.post('/api/intelligence/labs/community-intents/import', {
      data: {
        id: 'e2e-labs-intent',
        title: 'E2E Labs Intent',
        author: 'playwright',
        goal: 'cost-optimized',
      },
    });
    expect(res.ok()).toBeTruthy();
  });

  test('labs carbon API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/carbon');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('labs compliance report API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/compliance-report');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.export_format).toBe('json');
  });

  test('labs voice copilot API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/voice-copilot');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
  });

  test('labs graph export API', async ({ request }) => {
    const res = await request.get('/api/intelligence/labs/graph-export?format=neo4j');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.status).toBe('ship');
    expect(data.download_filename).toContain('cypher');
  });

  test('labs graduation panel on labs page', async ({ page }) => {
    await page.goto('/labs');
    await expect(page.getByTestId('labs-graduation-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('labs-overview-panel')).toBeVisible();
    await page.getByRole('button', { name: 'Terraform' }).click();
    await expect(page.getByTestId('labs-terraform-panel')).toBeVisible();
  });
});
