// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useCallback, useSyncExternalStore } from 'react';
import type { AppView } from '../types/api';
import { getPreferences, setPreferences, subscribePreferences, togglePinned } from '../utils/preferences';

export function usePreferences() {
  const preferences = useSyncExternalStore(subscribePreferences, getPreferences, getPreferences);
  const togglePin = useCallback((view: AppView) => {
    setPreferences({ pinnedViews: togglePinned(getPreferences().pinnedViews, view) });
  }, []);
  return { ...preferences, setPreferences, togglePin };
}
