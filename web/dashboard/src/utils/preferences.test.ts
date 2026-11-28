// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { DEFAULT_PREFERENCES, getPreferences, parsePreferences, setPreferences, togglePinned } from './preferences';

describe('preferences', () => {
  it('falls back to defaults for missing or broken storage', () => {
    expect(parsePreferences(null)).toEqual(DEFAULT_PREFERENCES);
    expect(parsePreferences('{not json')).toEqual(DEFAULT_PREFERENCES);
    expect(parsePreferences('null')).toEqual(DEFAULT_PREFERENCES);
    expect(parsePreferences('42')).toEqual(DEFAULT_PREFERENCES);
  });

  it('drops values of the wrong type and duplicate pins', () => {
    expect(parsePreferences(JSON.stringify({ pinnedViews: ['audit', 3, 'audit'], density: 'huge', sidebarCollapsed: 'yes' }))).toEqual({
      ...DEFAULT_PREFERENCES,
      pinnedViews: ['audit'],
    });
  });

  it('toggles a pin and keeps pin order', () => {
    expect(togglePinned([], 'audit')).toEqual(['audit']);
    expect(togglePinned(['audit', 'events'], 'audit')).toEqual(['events']);
  });

  it('keeps a change in memory when storage is unavailable', () => {
    setPreferences({ density: 'compact' });
    expect(getPreferences().density).toBe('compact');
    setPreferences({ density: 'comfortable' });
  });
});
