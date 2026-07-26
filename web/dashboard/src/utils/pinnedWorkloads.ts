// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

const STORAGE_KEY = 'aether_pinned_workloads';
const MAX_PINNED = 40;

function readPinned(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((value): value is string => typeof value === 'string' && value.length > 0);
  } catch {
    return [];
  }
}

function writePinned(names: string[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(names.slice(0, MAX_PINNED)));
  } catch {
    /* ignore storage errors */
  }
}

export function getPinnedWorkloads(): string[] {
  return readPinned();
}

export function isWorkloadPinned(name: string): boolean {
  return readPinned().includes(name);
}

/** Toggle pin state. Returns the next pinned set. */
export function togglePinnedWorkload(name: string): string[] {
  const current = readPinned();
  const next = current.includes(name)
    ? current.filter((entry) => entry !== name)
    : [name, ...current.filter((entry) => entry !== name)].slice(0, MAX_PINNED);
  writePinned(next);
  return next;
}
