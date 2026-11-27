// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { Check, Monitor, Moon, Rows3, Rows4, Sun } from 'lucide-react';
import { useTheme, type AppTheme } from '../contexts/ThemeContext';
import { usePreferences } from '../hooks/usePreferences';

const OPTIONS = [
  { value: 'light', label: 'Light', Icon: Sun },
  { value: 'dark', label: 'Dark', Icon: Moon },
  { value: 'system', label: 'System', Icon: Monitor },
] as const satisfies readonly { value: AppTheme; label: string; Icon: typeof Sun }[];

const DENSITIES = [
  { value: 'comfortable', label: 'Comfortable', Icon: Rows3 },
  { value: 'compact', label: 'Compact', Icon: Rows4 },
] as const;

const ITEM = 'flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm text-foreground hover:bg-hover focus:bg-hover';

/** Appearance menu: theme (Light, Dark or the operating system) and row density. */
export default function ThemeToggle() {
  const { theme, resolvedTheme, setTheme } = useTheme();
  const { density, setPreferences } = usePreferences();
  const [open, setOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const current = OPTIONS.find((o) => o.value === theme) ?? OPTIONS[2];
  const ButtonIcon = resolvedTheme === 'dark' ? Moon : Sun;

  useEffect(() => {
    // The first checked item is the current theme.
    if (open) menuRef.current?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
  }, [open]);

  const close = () => {
    setOpen(false);
    buttonRef.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape' || event.key === 'Tab') {
      event.stopPropagation();
      if (event.key === 'Escape') event.preventDefault();
      close();
      return;
    }
    const items = Array.from(menuRef.current?.querySelectorAll<HTMLElement>('[role="menuitemradio"]') ?? []);
    const index = items.indexOf(document.activeElement as HTMLElement);
    const next =
      event.key === 'ArrowDown' ? (index + 1) % items.length
      : event.key === 'ArrowUp' ? (index - 1 + items.length) % items.length
      : event.key === 'Home' ? 0
      : event.key === 'End' ? items.length - 1
      : -1;
    if (next < 0) return;
    event.preventDefault();
    items[next]?.focus();
  };

  return (
    <div className="relative">
      <button
        ref={buttonRef}
        type="button"
        data-testid="theme-toggle"
        aria-label={`Theme: ${current.label}`}
        title={`Theme: ${current.label}`}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        className="grid h-9 w-9 place-items-center rounded-full text-muted transition-colors hover:bg-hover hover:text-foreground"
      >
        <ButtonIcon size={17} aria-hidden />
      </button>
      {open ? (
        <>
          <div className="fixed inset-0 z-[58]" aria-hidden onClick={() => setOpen(false)} />
          <div
            ref={menuRef}
            role="menu"
            aria-label="Theme and density"
            onKeyDown={onKeyDown}
            className="glass-dropdown-surface absolute right-0 top-[calc(100%+8px)] z-[61] w-44 p-1.5"
          >
            {OPTIONS.map(({ value, label, Icon }) => (
              <button
                key={value}
                type="button"
                role="menuitemradio"
                aria-checked={theme === value}
                tabIndex={-1}
                onClick={() => {
                  setTheme(value);
                  close();
                }}
                className={ITEM}
              >
                <Icon size={15} className="text-muted" aria-hidden />
                <span className="flex-1">{label}</span>
                {theme === value ? <Check size={15} className="text-primary" aria-hidden /> : null}
              </button>
            ))}
            <div role="group" aria-label="Density" className="mt-1.5 border-t border-border pt-1.5">
              <p className="px-2.5 pb-0.5 text-[11px] font-semibold text-subtle" aria-hidden>Density</p>
              {DENSITIES.map(({ value, label, Icon }) => (
                <button
                  key={value}
                  type="button"
                  role="menuitemradio"
                  aria-checked={density === value}
                  tabIndex={-1}
                  onClick={() => {
                    setPreferences({ density: value });
                    close();
                  }}
                  className={ITEM}
                >
                  <Icon size={15} className="text-muted" aria-hidden />
                  <span className="flex-1">{label}</span>
                  {density === value ? <Check size={15} className="text-primary" aria-hidden /> : null}
                </button>
              ))}
            </div>
          </div>
        </>
      ) : null}
    </div>
  );
}
