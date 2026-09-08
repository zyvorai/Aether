// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getPinnedWorkloads, isWorkloadPinned, togglePinnedWorkload } from './pinnedWorkloads';

describe('pinnedWorkloads', () => {
  beforeEach(() => {
    const store = new Map<string, string>();
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => {
        store.set(key, value);
      },
      removeItem: (key: string) => {
        store.delete(key);
      },
    });
  });

  it('starts empty and pins/unpins workloads', () => {
    expect(getPinnedWorkloads()).toEqual([]);
    expect(togglePinnedWorkload('ns/pod-a')).toEqual(['ns/pod-a']);
    expect(isWorkloadPinned('ns/pod-a')).toBe(true);
    expect(togglePinnedWorkload('ns/pod-a')).toEqual([]);
    expect(isWorkloadPinned('ns/pod-a')).toBe(false);
  });

  it('keeps newest pin first and toggling removes existing pins', () => {
    togglePinnedWorkload('a');
    togglePinnedWorkload('b');
    expect(getPinnedWorkloads()).toEqual(['b', 'a']);
    togglePinnedWorkload('a');
    expect(getPinnedWorkloads()).toEqual(['b']);
    togglePinnedWorkload('a');
    expect(getPinnedWorkloads()).toEqual(['a', 'b']);
  });
});
