import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';

export type AppTheme = 'dark' | 'light' | 'steel';

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
  if (raw === 'light' || raw === 'steel' || raw === 'dark') return raw;
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
    root.classList.remove('light-theme', 'steel-theme');
    if (theme === 'light') root.classList.add('light-theme');
    else if (theme === 'steel') root.classList.add('steel-theme');
  }, [theme]);

  const toggleDarkLight = useCallback(() => {
    setThemeState((t) => {
      if (t === 'light') return 'dark';
      if (t === 'steel') return 'dark';
      return 'light';
    });
  }, []);

  return <ThemeContext.Provider value={{ theme, setTheme, toggleDarkLight }}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  return useContext(ThemeContext);
}
