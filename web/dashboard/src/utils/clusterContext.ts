// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

const STORAGE_KEY = 'aether_cluster_context';

export function getClusterContext(): string | null {
  try {
    const v = sessionStorage.getItem(STORAGE_KEY);
    return v && v.trim() !== '' ? v : null;
  } catch {
    return null;
  }
}

export function setClusterContext(context: string): void {
  try {
    const trimmed = context.trim();
    if (trimmed) {
      sessionStorage.setItem(STORAGE_KEY, trimmed);
    } else {
      sessionStorage.removeItem(STORAGE_KEY);
    }
    window.dispatchEvent(new CustomEvent('aether-cluster-context', { detail: trimmed }));
  } catch {
    /* sessionStorage unavailable */
  }
}

export function clearClusterContext(): void {
  setClusterContext('');
}
