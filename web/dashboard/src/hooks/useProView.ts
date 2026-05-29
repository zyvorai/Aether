// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';

const STORAGE_KEY = 'zyvor-pro-view';

export function useProView(): [boolean, (next: boolean) => void] {
  const [proView, setProViewState] = useState(() => {
    try {
      return localStorage.getItem(STORAGE_KEY) === '1';
    } catch {
      return false;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, proView ? '1' : '0');
    } catch {
      /* ignore */
    }
  }, [proView]);

  const setProView = useCallback((next: boolean) => {
    setProViewState(next);
  }, []);

  return [proView, setProView];
}
