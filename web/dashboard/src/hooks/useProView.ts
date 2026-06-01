// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { isMacOSShell } from '../utils/macosBridge';

const STORAGE_KEY = 'zyvor-pro-view';
const CLASSIC_KEY = 'zyvor-classic-nav';

function readStored(key: string, fallback: boolean): boolean {
  try {
    const stored = localStorage.getItem(key);
    if (stored === null) return fallback;
    return stored === '1';
  } catch {
    return fallback;
  }
}

export function useProView(): [boolean, (next: boolean) => void] {
  const [proView, setProViewState] = useState(() => {
    if (isMacOSShell()) return true;
    if (readStored(CLASSIC_KEY, false)) return false;
    return readStored(STORAGE_KEY, true);
  });

  useEffect(() => {
    if (isMacOSShell()) return;
    try {
      localStorage.setItem(STORAGE_KEY, proView ? '1' : '0');
      localStorage.setItem(CLASSIC_KEY, proView ? '0' : '1');
    } catch {
      /* ignore */
    }
  }, [proView]);

  const setProView = useCallback((next: boolean) => {
    if (isMacOSShell() && !next) return;
    setProViewState(next);
  }, []);

  return [isMacOSShell() ? true : proView, setProView];
}

export function useClassicNav(): [boolean, (next: boolean) => void] {
  const [classic, setClassic] = useState(() => readStored(CLASSIC_KEY, false));

  const setClassicNav = useCallback((next: boolean) => {
    setClassic(next);
    try {
      localStorage.setItem(CLASSIC_KEY, next ? '1' : '0');
      localStorage.setItem(STORAGE_KEY, next ? '0' : '1');
    } catch {
      /* ignore */
    }
    window.location.reload();
  }, []);

  return [classic, setClassicNav];
}
