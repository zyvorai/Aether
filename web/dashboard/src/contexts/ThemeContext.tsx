// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';

export type AppTheme = 'light' | 'dark' | 'system';

interface ThemeContextType {
  theme: AppTheme;
  resolvedTheme: 'light' | 'dark';
  setTheme: (t: AppTheme) => void;
  toggleDarkLight: () => void;
}

const ThemeContext = createContext<ThemeContextType>({
  theme: 'system',
  resolvedTheme: 'dark',
  setTheme: () => {},
  toggleDarkLight: () => {},
});

function parseStoredTheme(raw: string | null): AppTheme {
  if (raw === 'steel') return 'dark';
  if (raw === 'aurora') return 'light';
  if (raw === 'light' || raw === 'dark' || raw === 'system') return raw;
  return 'system';
}

function getSystemPrefersDark(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<AppTheme>(() =>
    parseStoredTheme(typeof localStorage !== 'undefined' ? localStorage.getItem('aether-theme') : null),
  );
  const [systemPrefersDark, setSystemPrefersDark] = useState(getSystemPrefersDark);

  const setTheme = useCallback((t: AppTheme) => {
    setThemeState(t);
  }, []);

  useEffect(() => {
    localStorage.setItem('aether-theme', theme);
    const root = document.documentElement;
    if (theme === 'system') root.removeAttribute('data-theme');
    else root.setAttribute('data-theme', theme);
  }, [theme]);

  useEffect(() => {
    const mql = window.matchMedia('(prefers-color-scheme: dark)');
    const onChange = () => setSystemPrefersDark(mql.matches);
    mql.addEventListener('change', onChange);
    return () => mql.removeEventListener('change', onChange);
  }, []);

  const resolvedTheme: 'light' | 'dark' = theme === 'system' ? (systemPrefersDark ? 'dark' : 'light') : theme;

  const toggleDarkLight = useCallback(() => {
    setThemeState((t) => {
      const current = t === 'system' ? (getSystemPrefersDark() ? 'dark' : 'light') : t;
      return current === 'dark' ? 'light' : 'dark';
    });
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme, toggleDarkLight }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  return useContext(ThemeContext);
}
