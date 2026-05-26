// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';
import { cycleTheme } from '../utils/themeSurface';

export type AppTheme = 'dark' | 'steel' | 'aurora';

interface ThemeContextType {
  theme: AppTheme;
  setTheme: (t: AppTheme) => void;
  toggleDarkLight: () => void;
}

const ThemeContext = createContext<ThemeContextType>({
  theme: 'dark',
  setTheme: () => {},
  toggleDarkLight: () => {},
});

function parseStoredTheme(raw: string | null): AppTheme {
  if (raw === 'light') return 'aurora';
  if (raw === 'steel' || raw === 'aurora' || raw === 'dark') return raw;
  return 'dark';
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<AppTheme>(() =>
    parseStoredTheme(typeof localStorage !== 'undefined' ? localStorage.getItem('aether-theme') : null),
  );

  const setTheme = useCallback((t: AppTheme) => {
    setThemeState(t);
  }, []);

  useEffect(() => {
    localStorage.setItem('aether-theme', theme);
    const root = document.documentElement;
    root.classList.remove('steel-theme', 'aurora-theme');
    if (theme === 'steel') root.classList.add('steel-theme');
    else if (theme === 'aurora') root.classList.add('aurora-theme');
  }, [theme]);

  const toggleDarkLight = useCallback(() => {
    setThemeState((t) => cycleTheme(t));
  }, []);

  return <ThemeContext.Provider value={{ theme, setTheme, toggleDarkLight }}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  return useContext(ThemeContext);
}
