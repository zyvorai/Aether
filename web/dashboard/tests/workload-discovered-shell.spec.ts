// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect } from './fixtures';

const discoveredVm = {
  name: 'dev/default/demo-vm',
  status: 'running',
  runtime: 'KubeVirt',
  image: 'vm:demo-vm',
  source: 'cluster',
  cluster: 'dev',
  namespace: 'default',
  kind: 'VirtualMachine',
  created_at: '2026-01-01T00:00:00Z',
};

test.describe('Discovered workload Shell', () => {
  test('resolves virt-launcher pod name for exec WebSocket', async ({ page }) => {
    await page.addInitScript(() => {
      const Orig = window.WebSocket;
      const Wrapped = function (this: unknown, url: string | URL, protocols?: string | string[]) {
        const href = String(url);
        if (href.includes('/cluster/ws/exec')) {
          (window as unknown as { __lastShellWsUrl?: string }).__lastShellWsUrl = href;
          const fake: {
            readyState: number;
            onopen: ((ev: object) => void) | null;
            onmessage: ((ev: { data: string }) => void) | null;
            onclose: ((ev: object) => void) | null;
            onerror: ((ev: object) => void) | null;
            send: () => void;
            close: () => void;
            addEventListener: () => void;
            removeEventListener: () => void;
          } = {
            readyState: Orig.OPEN,
            onopen: null,
            onmessage: null,
            onclose: null,
            onerror: null,
            send() {},
            close() {
              this.readyState = Orig.CLOSED;
              this.onclose?.({});
            },
            addEventListener() {},
            removeEventListener() {},
          };
          queueMicrotask(() => {
            fake.onopen?.({});
            fake.onmessage?.({ data: '[fake] ready\n' });
          });
          return fake;
        }
        return protocols === undefined ? new Orig(url) : new Orig(url, protocols);
      } as unknown as typeof WebSocket;
      Wrapped.prototype = Orig.prototype;
      Wrapped.CONNECTING = Orig.CONNECTING;
      Wrapped.OPEN = Orig.OPEN;
      Wrapped.CLOSING = Orig.CLOSING;
      Wrapped.CLOSED = Orig.CLOSED;
      window.WebSocket = Wrapped;
    });

    await page.route('**/api/workloads', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: [discoveredVm] }),
      }),
    );
    await page.route('**/api/cluster/resource**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            cluster: 'dev',
            namespace: 'default',
            kind: 'VirtualMachine',
            name: 'demo-vm',
            api_version: 'kubevirt.io/v1',
            pods: [
              { name: 'virt-launcher-demo-pending', phase: 'Pending', ready: 0, total_containers: 1, restarts: 0, node: null },
              { name: 'virt-launcher-demo-running', phase: 'Running', ready: 1, total_containers: 1, restarts: 0, node: 'node-1' },
            ],
            conditions: [],
            owner_references: [],
            owned_resources: [],
            manifest: { apiVersion: 'kubevirt.io/v1', kind: 'VirtualMachine', metadata: { name: 'demo-vm' } },
          },
        }),
      }),
    );
    await page.route('**/api/cluster/logs**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: 'launcher log line\n' }),
      }),
    );

    await page.goto(`/workloads?workload=${encodeURIComponent(discoveredVm.name)}`);

    await expect(page.getByTestId('workload-shell-button')).toBeVisible({ timeout: 20_000 });
    await page.getByTestId('workload-shell-button').click();

    await expect(page.getByTestId('workload-shell-title')).toContainText('virt-launcher-demo-running', {
      timeout: 10_000,
    });

    const wsUrl = await page.evaluate(() => (window as unknown as { __lastShellWsUrl?: string }).__lastShellWsUrl);
    expect(wsUrl).toBeTruthy();
    expect(wsUrl!).toContain('pod=virt-launcher-demo-running');
    expect(wsUrl!).not.toContain(encodeURIComponent(discoveredVm.name));
  });
});
