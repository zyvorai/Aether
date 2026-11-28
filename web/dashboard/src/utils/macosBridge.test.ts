// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { afterEach, describe, expect, it, vi } from 'vitest';
import { refreshMacOSOfflineCache } from './macosBridge';

describe('macOS offline cache', () => {
  afterEach(() => vi.unstubAllGlobals());

  const browser = (tauri: boolean) => {
    const posted = vi.fn(async () => new Response('{"success":true,"data":null,"error":null}'));
    vi.stubGlobal('fetch', posted);
    vi.stubGlobal('sessionStorage', { getItem: () => null });
    vi.stubGlobal('localStorage', { getItem: () => null });
    vi.stubGlobal('window', tauri ? { __TAURI__: { core: { invoke: async () => {} } } } : {});
    return posted;
  };

  it('writes nothing when a browser only looks at the Overview', async () => {
    const posted = browser(false);
    await refreshMacOSOfflineCache();
    expect(posted).not.toHaveBeenCalled();
  });

  it('is still refreshed by the macOS app', async () => {
    const posted = browser(true);
    await refreshMacOSOfflineCache();
    expect(posted).toHaveBeenCalledTimes(1);
  });
});
