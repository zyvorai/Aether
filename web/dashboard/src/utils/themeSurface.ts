// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

export function appShellClass(): string {
  return 'app-shell min-h-screen flex flex-col bg-canvas text-ink';
}

export function navbarShellClass(): string {
  return 'sticky top-0 z-40 border-b border-rule bg-glass backdrop-blur-xl backdrop-saturate-[1.8]';
}

export function dropdownSurfaceClass(): string {
  return 'border border-rule bg-raised shadow-ambient';
}

export function dropdownItemClass(active: boolean): string {
  return active ? 'text-brand bg-brand-wash' : 'text-ink-2 hover:text-ink hover:bg-hover';
}
