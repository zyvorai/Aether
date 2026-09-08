// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

export function appShellClass(): string {
  return 'app-shell min-h-screen flex flex-col bg-background text-foreground';
}

export function navbarShellClass(): string {
  return 'sticky top-0 z-40 border-b border-rule bg-glass backdrop-blur-xl backdrop-saturate-[1.8]';
}

export function dropdownSurfaceClass(): string {
  return 'border border-rule bg-surface-elevated shadow-ambient';
}

export function dropdownItemClass(active: boolean): string {
  return active ? 'text-primary bg-primary-wash' : 'text-muted hover:text-foreground hover:bg-hover';
}
