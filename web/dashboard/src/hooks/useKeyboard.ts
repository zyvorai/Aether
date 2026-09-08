// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useCallback } from 'react';

interface KeyboardConfig {
  onRefresh: () => void;
  onCommandPalette: () => void;
  /** ? — help dialog (shortcuts & about). */
  onOpenHelp?: () => void;
  enabled: boolean;
}

export function useKeyboard({ onRefresh, onCommandPalette, onOpenHelp, enabled }: KeyboardConfig) {
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (!enabled) return;

    // Cmd/Ctrl + K = command palette. Checked before the input/textarea guard below
    // so it really does work everywhere, including while typing in a page's own
    // search/filter field.
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      onCommandPalette();
      return;
    }

    // Ignore single-key shortcuts while typing in an input/textarea
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return;

    // Single-key shortcuts (only when not in input fields)
    switch (e.key) {
      case 'r':
        onRefresh();
        break;
      case '?':
        if (onOpenHelp) {
          e.preventDefault();
          onOpenHelp();
        }
        break;
    }
  }, [enabled, onRefresh, onCommandPalette, onOpenHelp]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}
