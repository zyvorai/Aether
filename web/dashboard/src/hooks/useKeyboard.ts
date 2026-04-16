import { useEffect, useCallback } from 'react';

interface KeyboardConfig {
  onRefresh: () => void;
  onCommandPalette: () => void;
  enabled: boolean;
}

export function useKeyboard({ onRefresh, onCommandPalette, enabled }: KeyboardConfig) {
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (!enabled) return;

    // Ignore if typing in an input/textarea
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return;

    // Cmd/Ctrl + K = command palette (handled separately since it should work everywhere)
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      onCommandPalette();
      return;
    }

    // Single-key shortcuts (only when not in input fields)
    switch (e.key) {
      case 'r': onRefresh(); break;
      case '?': onCommandPalette(); break;
    }
  }, [enabled, onRefresh, onCommandPalette]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}
