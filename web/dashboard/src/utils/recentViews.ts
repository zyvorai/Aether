// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { AppView } from '../types/api';

const STORAGE_KEY = 'aether_recent_views';
const MAX_RECENT = 5;

export function pushRecentView(view: AppView): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    const existing = Array.isArray(parsed) ? parsed.filter((v): v is AppView => typeof v === 'string') : [];
    const next = [view, ...existing.filter((v) => v !== view)].slice(0, MAX_RECENT);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* ignore storage errors */
  }
}

export function getRecentViews(): AppView[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((v): v is AppView => typeof v === 'string');
  } catch {
    return [];
  }
}
