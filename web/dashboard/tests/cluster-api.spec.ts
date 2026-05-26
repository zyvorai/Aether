// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect } from '@playwright/test';

test.describe('Cluster API (no live cluster required)', () => {
  test('port-forward stop returns not found for unknown session', async ({ request }) => {
    const res = await request.post('/api/cluster/port-forward/stop', {
      data: { session_id: 'nonexistent-session-id' },
    });
    expect(res.status()).toBe(404);
  });

  test('OPA policy check allows when OPA not configured', async ({ request }) => {
    const res = await request.post('/api/policy/opa', {
      data: {
        spec: {
          apiVersion: 'aether/v1',
          kind: 'Workload',
          metadata: { name: 'opa-test', owner: 'ci', project: 'ci' },
          build: { context: '.', dockerfile: 'Dockerfile', registry: 'local/test' },
          requirements: { cpu: '1', memory: '512Mi', storage: '1Gi' },
          runtime: { preferred: 'auto', allow: ['container'] },
        },
      },
    });
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data?.allowed).toBe(true);
  });

  test('observability summary returns structured payload', async ({ request }) => {
    const res = await request.get('/api/observability/summary');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    const data = body.data ?? body;
    expect(typeof data.api_http_requests_total).toBe('number');
    expect(typeof data.prometheus_configured).toBe('boolean');
  });

  test('cilium status returns expected shape or cluster error', async ({ request }) => {
    const res = await request.get('/api/cluster/cilium/status');
    expect([200, 500]).toContain(res.status());
    if (res.ok()) {
      const body = await res.json();
      const data = body.data ?? body;
      expect(data).toHaveProperty('cni');
      expect(data).toHaveProperty('connectivity_check');
    }
  });
});
