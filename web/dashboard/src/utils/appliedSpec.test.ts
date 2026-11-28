// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import { loadAppliedSpec } from './appliedSpec';

function serve(routes: Record<string, unknown>) {
  const asked: string[] = [];
  vi.stubGlobal('sessionStorage', { getItem: () => null });
  vi.stubGlobal('localStorage', { getItem: () => null });
  vi.stubGlobal('fetch', async (url: string) => {
    asked.push(url);
    const found = url in routes;
    return new Response(JSON.stringify(found ? { success: true, data: routes[url], error: null } : { success: false, data: null, error: 'nope' }), {
      status: found ? 200 : 500,
    });
  });
  return asked;
}

describe('applied spec of a managed workload', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('shows the newest recorded revision', async () => {
    const asked = serve({
      '/api/workloads/web/revisions': [{ id: 1 }, { id: 2 }],
      '/api/workloads/web/revisions/2': { id: 2, spec_yaml: 'kind: Workload\n' },
    });
    expect(await loadAppliedSpec('web')).toMatchObject({ state: 'recorded', revision: { id: 2, spec_yaml: 'kind: Workload\n' } });
    expect(asked).toEqual(['/api/workloads/web/revisions', '/api/workloads/web/revisions/2']);
  });

  it('tells nothing recorded apart from nothing loaded', async () => {
    serve({ '/api/workloads/web/revisions': [] });
    expect(await loadAppliedSpec('web')).toEqual({ state: 'none' });
    serve({});
    expect((await loadAppliedSpec('web')).state).toBe('failed');
  });
});
