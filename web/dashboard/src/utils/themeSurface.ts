// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { AppTheme } from '../contexts/ThemeContext';

export const THEME_OPTIONS: { value: AppTheme; label: string }[] = [
  { value: 'dark', label: 'Dark' },
  { value: 'steel', label: 'Steel' },
  { value: 'aurora', label: 'Aurora' },
];

export function cycleTheme(current: AppTheme): AppTheme {
  const order: AppTheme[] = ['dark', 'steel', 'aurora'];
  const idx = order.indexOf(current);
  return order[(idx + 1) % order.length];
}

export function appShellClass(theme: AppTheme): string {
  switch (theme) {
    case 'steel':
      return 'dashboard-steel min-h-screen flex flex-col text-[#d7dde5] steel-grid';
    case 'aurora':
      return 'dashboard-aurora min-h-screen flex flex-col text-slate-100';
    default:
      return 'app-shell min-h-screen flex flex-col text-slate-100 steel-grid';
  }
}

export function navbarShellClass(theme: AppTheme): string {
  switch (theme) {
    case 'steel':
      return 'border-[rgba(140,160,190,0.18)] bg-gradient-to-b from-[#0f141a] via-[#1a222d] to-[#0c1117] shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_10px_30px_rgba(0,0,0,0.45)]';
    case 'aurora':
      return 'navbar-aurora border-violet-500/25';
    default:
      return 'navbar-blur navbar-glass-accent relative border-slate-800/40';
  }
}

export function loginPageClass(theme: AppTheme): string {
  if (theme === 'steel') return 'login-page-steel';
  if (theme === 'aurora') return 'login-page-aurora';
  return '';
}

export function themeSelectClass(theme: AppTheme): string {
  const base =
    'rounded-lg border px-2 py-1 text-xs outline-none focus-visible:ring-2 focus-visible:ring-aether/40';
  switch (theme) {
    case 'steel':
      return `${base} nav-steel-select text-[#d7dde5]`;
    case 'aurora':
      return `${base} nav-aurora-select text-violet-100`;
    default:
      return `${base} border-slate-700/80 bg-slate-900/60 text-slate-200`;
  }
}

export function dropdownSurfaceClass(_theme: AppTheme): string {
  return 'glass-dropdown-surface';
}

export function dropdownItemClass(active: boolean, _theme: AppTheme): string {
  return active
    ? 'text-aether bg-aether/10'
    : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80';
}
