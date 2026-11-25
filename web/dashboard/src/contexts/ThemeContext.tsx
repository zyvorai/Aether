// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';

export type AppTheme = 'light' | 'dark' | 'system';

interface ThemeContextType {
  theme: AppTheme;
  resolvedTheme: 'light' | 'dark';
  setTheme: (t: AppTheme) => void;
  toggleDarkLight: () => void;
  /** @deprecated Accent picker removed — single accent. Kept for compile safety. */
  accent: 'blue';
  setAccent: (a: 'blue') => void;
}

const ThemeContext = createContext<ThemeContextType>({
  theme: 'system',
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
  return 'system';
}

function getSystemPrefersDark(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function readStoredTheme(): string | null {
  try {
    return localStorage.getItem('aether-theme');
  } catch {
    return null;
  }
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<AppTheme>(() =>
    parseStoredTheme(readStoredTheme()),
  );
  const [systemPrefersDark, setSystemPrefersDark] = useState(getSystemPrefersDark);

  const setTheme = useCallback((t: AppTheme) => {
    setThemeState(t);
  }, []);

  const setAccent = useCallback((_a: 'blue') => {
    /* no-op — the accent is fixed */
  }, []);

  const resolvedTheme: 'light' | 'dark' = theme === 'system' ? (systemPrefersDark ? 'dark' : 'light') : theme;

  // Mirrors the inline script in index.html, which applies the theme before first paint.
  useEffect(() => {
    try {
      localStorage.setItem('aether-theme', theme);
    } catch {
      /* storage may be unavailable; the choice then lasts for this page only */
    }
    const root = document.documentElement;
    root.removeAttribute('data-accent');
    root.setAttribute('data-theme', resolvedTheme);
    root.classList.toggle('dark-theme', resolvedTheme === 'dark');
    root.classList.toggle('light-theme', resolvedTheme === 'light');
    document
      .querySelector('meta[name="theme-color"]')
      ?.setAttribute('content', getComputedStyle(root).getPropertyValue('--background').trim());
  }, [theme, resolvedTheme]);

  useEffect(() => {
    const mql = window.matchMedia('(prefers-color-scheme: dark)');
    const onChange = () => setSystemPrefersDark(mql.matches);
    mql.addEventListener('change', onChange);
    return () => mql.removeEventListener('change', onChange);
  }, []);

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
  { value: 'blue', label: 'Blue', swatchClass: 'bg-accent-fill' },
];

export type AccentColor = 'blue';
