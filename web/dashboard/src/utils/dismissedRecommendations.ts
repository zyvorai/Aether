// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

const STORAGE_KEY = 'aether-dismissed-platform-recs';

export function getDismissedRecommendations(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new Set();
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return new Set();
    return new Set(parsed.filter((id): id is string => typeof id === 'string'));
  } catch {
    return new Set();
  }
}

export function dismissRecommendation(id: string): void {
  const next = getDismissedRecommendations();
  next.add(id);
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...next]));
}
