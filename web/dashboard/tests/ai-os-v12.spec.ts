// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('AI Infrastructure OS v12 — Copilot & LLM', () => {
  test('copilot memory API', async ({ request }) => {
    const res = await request.get('/api/intelligence/copilot/memory');
    expect(res.ok()).toBeTruthy();
  });

  test('copilot route API', async ({ request }) => {
    const res = await request.post('/api/intelligence/copilot/route', {
      data: { message: 'Find cost savings across the fleet' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.agent).toBe('cost');
  });

  test('copilot llm status API', async ({ request }) => {
    const res = await request.get('/api/intelligence/copilot/llm-status');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.active_provider).toBeTruthy();
  });

  test('copilot voice lab API', async ({ request }) => {
    const res = await request.get('/api/intelligence/copilot/voice-lab');
    expect(res.ok()).toBeTruthy();
  });

  test('copilot runbook API', async ({ request }) => {
    const res = await request.post('/api/intelligence/copilot/runbook', {
      data: { prompt: 'Restart unhealthy pods safely' },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(data.markdown).toContain('Restart unhealthy pods');
  });

  test('copilot policy explain API', async ({ request }) => {
    const res = await request.post('/api/intelligence/copilot/policy-explain', { data: {} });
    expect(res.ok()).toBeTruthy();
  });

  test('copilot audit API', async ({ request }) => {
    const res = await request.get('/api/intelligence/copilot/audit');
    expect(res.ok()).toBeTruthy();
  });

  test('copilot rbac scopes API', async ({ request }) => {
    const res = await request.get('/api/intelligence/copilot/rbac-scopes');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(Array.isArray(data.tools)).toBeTruthy();
  });

  test('copilot confirm batch API rejects empty', async ({ request }) => {
    const res = await request.post('/api/copilot/confirm-batch', {
      data: { session_id: '', action_ids: [] },
    });
    expect(res.status()).toBe(400);
  });

  test('copilot platform panel on copilot page', async ({ page }) => {
    await page.goto('/copilot');
    await expect(page.getByTestId('copilot-platform-panel')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('copilot-memory-panel')).toBeVisible();
    await page.getByRole('button', { name: 'Agents' }).click();
    await expect(page.getByTestId('copilot-route-panel')).toBeVisible();
    await page.getByTestId('copilot-route-button').click();
    await expect(page.getByText('FinOps Agent')).toBeVisible({ timeout: 10_000 });
  });
});
