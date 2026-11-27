// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * Per-viewer interface preferences, kept in this browser only. Every storage
 * call is guarded: private windows and blocked storage must not break the UI.
 */

import type { AppView } from '../types/api';

export type Density = 'comfortable' | 'compact';

export interface Preferences {
  /** Pages pinned to the top of the sidebar and the command palette. */
  pinnedViews: AppView[];
  density: Density;
  /** Desktop sidebar collapsed to an icon rail. */
  sidebarCollapsed: boolean;
  /** The first-visit orientation card has been closed. */
  welcomeDismissed: boolean;
}

export const PREFERENCES_KEY = 'aether-ui-preferences';

export const DEFAULT_PREFERENCES: Preferences = {
  pinnedViews: [],
  density: 'comfortable',
  sidebarCollapsed: false,
  welcomeDismissed: false,
};

export function parsePreferences(raw: string | null): Preferences {
  if (!raw) return DEFAULT_PREFERENCES;
  try {
    const parsed = JSON.parse(raw) as Partial<Record<keyof Preferences, unknown>> | null;
    if (!parsed || typeof parsed !== 'object') return DEFAULT_PREFERENCES;
    return {
      pinnedViews: Array.isArray(parsed.pinnedViews)
        ? [...new Set(parsed.pinnedViews.filter((view): view is AppView => typeof view === 'string'))]
        : [],
      density: parsed.density === 'compact' ? 'compact' : 'comfortable',
      sidebarCollapsed: parsed.sidebarCollapsed === true,
      welcomeDismissed: parsed.welcomeDismissed === true,
    };
  } catch {
    return DEFAULT_PREFERENCES;
  }
}

export function togglePinned(pinned: AppView[], view: AppView): AppView[] {
  return pinned.includes(view) ? pinned.filter((v) => v !== view) : [...pinned, view];
}

const listeners = new Set<() => void>();
let current: Preferences | null = null;

function read(): Preferences {
  try {
    return parsePreferences(localStorage.getItem(PREFERENCES_KEY));
  } catch {
    return DEFAULT_PREFERENCES;
  }
}

export function getPreferences(): Preferences {
  current ??= read();
  return current;
}

export function setPreferences(patch: Partial<Preferences>): void {
  current = { ...getPreferences(), ...patch };
  try {
    localStorage.setItem(PREFERENCES_KEY, JSON.stringify(current));
  } catch {
    /* the choice then lasts for this page only */
  }
  listeners.forEach((listener) => listener());
}

export function subscribePreferences(listener: () => void): () => void {
  listeners.add(listener);
  const onStorage = (event: StorageEvent) => {
    if (event.key !== PREFERENCES_KEY) return;
    current = read();
    listener();
  };
  if (typeof window !== 'undefined') window.addEventListener('storage', onStorage);
  return () => {
    listeners.delete(listener);
    if (typeof window !== 'undefined') window.removeEventListener('storage', onStorage);
  };
}
