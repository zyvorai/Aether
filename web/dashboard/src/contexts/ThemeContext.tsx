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
  /** @deprecated Accent picker removed — Apple blue only. Kept for compile safety. */
  accent: 'blue';
  setAccent: (a: 'blue') => void;
}

const ThemeContext = createContext<ThemeContextType>({
  theme: 'light',
  resolvedTheme: 'light',
  setTheme: () => {},
  toggleDarkLight: () => {},
  accent: 'blue',
  setAccent: () => {},
});

function parseStoredTheme(raw: string | null): AppTheme {
  if (raw === 'steel') return 'dark';
  if (raw === 'aurora') return 'light';
  if (raw === 'light' || raw === 'dark' || raw === 'system') return raw;
  return 'light';
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

  const setAccent = useCallback((_a: 'blue') => {
    /* no-op — brand is fixed Apple blue */
  }, []);

  useEffect(() => {
    localStorage.setItem('aether-theme', theme);
    const root = document.documentElement;
    root.removeAttribute('data-accent');
    const resolved = theme === 'system' ? (getSystemPrefersDark() ? 'dark' : 'light') : theme;
    if (theme === 'system') root.removeAttribute('data-theme');
    else root.setAttribute('data-theme', theme);
    root.classList.toggle('dark-theme', resolved === 'dark');
    root.classList.toggle('light-theme', resolved === 'light');
  }, [theme, systemPrefersDark]);

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
    <ThemeContext.Provider
      value={{ theme, resolvedTheme, setTheme, toggleDarkLight, accent: 'blue', setAccent }}
    >
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  return useContext(ThemeContext);
}

/** @deprecated Removed with Aurora UX — kept so old imports compile until cleaned. */
export const ACCENT_OPTIONS: { value: 'blue'; label: string; swatchClass: string }[] = [
  { value: 'blue', label: 'Apple Blue', swatchClass: 'bg-primary' },
];

export type AccentColor = 'blue';
