// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';

export type AppTheme = 'light' | 'dark' | 'system';

/** iPhone 17 categorical accent choice — independent of light/dark. 'orange'
 * (Cosmic Orange, the default Zyvor brand) needs no data-accent override;
 * the others map to the [data-accent="..."] blocks in theme.css. */
export type AccentColor = 'orange' | 'deepblue' | 'sage' | 'mistblue' | 'lavender';

export const ACCENT_OPTIONS: { value: AccentColor; label: string; swatchClass: string }[] = [
  { value: 'orange', label: 'Cosmic Orange', swatchClass: 'bg-brand' },
  { value: 'deepblue', label: 'Deep Blue', swatchClass: 'bg-deepblue' },
  { value: 'sage', label: 'Sage', swatchClass: 'bg-sage' },
  { value: 'mistblue', label: 'Mist Blue', swatchClass: 'bg-mistblue' },
  { value: 'lavender', label: 'Lavender', swatchClass: 'bg-lavender' },
];

interface ThemeContextType {
  theme: AppTheme;
  resolvedTheme: 'light' | 'dark';
  setTheme: (t: AppTheme) => void;
  toggleDarkLight: () => void;
  accent: AccentColor;
  setAccent: (a: AccentColor) => void;
}

const ThemeContext = createContext<ThemeContextType>({
  theme: 'system',
  resolvedTheme: 'dark',
  setTheme: () => {},
  toggleDarkLight: () => {},
  accent: 'orange',
  setAccent: () => {},
});

function parseStoredTheme(raw: string | null): AppTheme {
  if (raw === 'steel') return 'dark';
  if (raw === 'aurora') return 'light';
  if (raw === 'light' || raw === 'dark' || raw === 'system') return raw;
  return 'system';
}

function parseStoredAccent(raw: string | null): AccentColor {
  if (raw === 'deepblue' || raw === 'sage' || raw === 'mistblue' || raw === 'lavender' || raw === 'orange') return raw;
  return 'orange';
}

function getSystemPrefersDark(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<AppTheme>(() =>
    parseStoredTheme(typeof localStorage !== 'undefined' ? localStorage.getItem('aether-theme') : null),
  );
  const [accent, setAccentState] = useState<AccentColor>(() =>
    parseStoredAccent(typeof localStorage !== 'undefined' ? localStorage.getItem('aether-accent') : null),
  );
  const [systemPrefersDark, setSystemPrefersDark] = useState(getSystemPrefersDark);

  const setTheme = useCallback((t: AppTheme) => {
    setThemeState(t);
  }, []);

  const setAccent = useCallback((a: AccentColor) => {
    setAccentState(a);
  }, []);

  useEffect(() => {
    localStorage.setItem('aether-theme', theme);
    const root = document.documentElement;
    if (theme === 'system') root.removeAttribute('data-theme');
    else root.setAttribute('data-theme', theme);
  }, [theme]);

  useEffect(() => {
    localStorage.setItem('aether-accent', accent);
    const root = document.documentElement;
    if (accent === 'orange') root.removeAttribute('data-accent');
    else root.setAttribute('data-accent', accent);
  }, [accent]);

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
    <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme, toggleDarkLight, accent, setAccent }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  return useContext(ThemeContext);
}
