// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { agoLabel, liveState } from './liveState';

const now = 1_000_000;
const base = { connected: true, paused: false, heldUpdates: 0, lastRefreshed: new Date(now - 42_000), now };

describe('liveState', () => {
  it('formats ages', () => {
    expect(agoLabel(now - 3_000, now)).toBe('just now');
    expect(agoLabel(now - 90_000, now)).toBe('1m ago');
    expect(agoLabel(now - 7_300_000, now)).toBe('2h ago');
    expect(agoLabel(now + 5_000, now)).toBe('just now');
  });

  it('does not claim to be live while paused or disconnected', () => {
    expect(liveState(base)).toMatchObject({ tone: 'live', label: 'Live, 42s ago' });
    expect(liveState({ ...base, connected: false }).tone).toBe('offline');
    expect(liveState({ ...base, paused: true, heldUpdates: 3 })).toMatchObject({ tone: 'paused', label: 'Paused, 3 held' });
    expect(liveState({ ...base, paused: true, connected: false }).tone).toBe('paused');
  });
});
