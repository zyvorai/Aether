// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useCallback } from 'react';

interface KeyboardConfig {
  onRefresh: () => void;
  onCommandPalette: () => void;
  /** ? — help dialog (shortcuts & about). */
  onOpenHelp?: () => void;
  onToggleSidebar?: () => void;
  onTogglePin?: () => void;
  enabled: boolean;
}

export function useKeyboard({ onRefresh, onCommandPalette, onOpenHelp, onToggleSidebar, onTogglePin, enabled }: KeyboardConfig) {
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

    // Single keys stay out of the way of typing, browser chords (Cmd+R) and open dialogs.
    if (e.metaKey || e.ctrlKey || e.altKey || e.defaultPrevented) return;
    const target = e.target as HTMLElement;
    if (['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName) || target.isContentEditable) return;
    // "?" toggles the help dialog, so it must work while that dialog is open.
    if (e.key !== '?' && document.querySelector('[aria-modal="true"]')) return;
    if (e.key === '?' && target.closest?.('[aria-modal="true"]:not([aria-label="Help"])')) return;

    // Single-key shortcuts (only when not in input fields)
    switch (e.key) {
      case 'r':
        onRefresh();
        break;
      case '[':
        onToggleSidebar?.();
        break;
      case 'p':
        onTogglePin?.();
        break;
      case '/': {
        const field = document.querySelector<HTMLInputElement>(
          '#main-content input[type="search"], #main-content input[placeholder^="Search"], #main-content input[placeholder^="Filter"]',
        );
        if (field) {
          e.preventDefault();
          field.focus();
          field.select();
        }
        break;
      }
      case '?':
        if (onOpenHelp) {
          e.preventDefault();
          onOpenHelp();
        }
        break;
    }
  }, [enabled, onRefresh, onCommandPalette, onOpenHelp, onToggleSidebar, onTogglePin]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}
