// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import { loadSlaTargets } from './slaTargets';

const reply = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

describe('SLA targets', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('asks once for the list, never once per workload', async () => {
    const asked: string[] = [];
    vi.stubGlobal('sessionStorage', { getItem: () => null });
    vi.stubGlobal('localStorage', { getItem: () => null });
    vi.stubGlobal('fetch', async (url: string) => {
      asked.push(url);
      return reply(200, { success: true, data: [{ workload: 'web', uptime_target_pct: 99.9 }], error: null });
    });
    const targets = await loadSlaTargets();
    expect(asked).toEqual(['/api/sla']);
    expect(targets?.web.uptime_target_pct).toBe(99.9);
    expect(targets?.api).toBeUndefined();
  });

  it('does not read a failed request as no targets', async () => {
    vi.stubGlobal('sessionStorage', { getItem: () => null });
    vi.stubGlobal('localStorage', { getItem: () => null });
    vi.stubGlobal('fetch', async () => reply(500, { success: false, data: null, error: 'boom' }));
    expect(await loadSlaTargets()).toBeNull();
  });
});
