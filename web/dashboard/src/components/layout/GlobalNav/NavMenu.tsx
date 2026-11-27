// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { useLocation } from 'react-router';
import styles from './GlobalNav.module.css';

interface NavMenuProps {
  /** Accessible name of the trigger. */
  label: string;
  trigger: ReactNode;
  triggerClassName?: string;
  /** A menu holds actions; a panel holds information with its own controls. */
  kind?: 'menu' | 'dialog';
  title?: string;
  children: (close: () => void) => ReactNode;
}

/** A button with a drop-down: Escape and outside clicks close it, focus returns to the button. */
export default function NavMenu({ label, trigger, triggerClassName, kind = 'menu', title, children }: NavMenuProps) {
  const [open, setOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const { pathname } = useLocation();

  useEffect(() => setOpen(false), [pathname]);

  useEffect(() => {
    if (!open) return;
    const popup = popupRef.current;
    (popup?.querySelector<HTMLElement>('[role="menuitem"], button:not(:disabled), a') ?? popup)?.focus();
  }, [open]);

  const close = () => {
    setOpen(false);
    buttonRef.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      close();
      return;
    }
    if (kind !== 'menu') return;
    if (event.key === 'Tab') {
      close();
      return;
    }
    const items = Array.from(popupRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []);
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
    <div className={styles.menuWrap}>
      <button
        ref={buttonRef}
        type="button"
        className={triggerClassName ?? styles.icon}
        onClick={() => setOpen((value) => !value)}
        aria-haspopup={kind}
        aria-expanded={open}
        aria-label={label}
        title={label}
      >
        {trigger}
      </button>
      {open ? (
        <>
          <div className="fixed inset-0 z-[58] cursor-default" aria-hidden onClick={() => setOpen(false)} />
          <div
            ref={popupRef}
            role={kind}
            aria-label={title ?? label}
            tabIndex={-1}
            onKeyDown={onKeyDown}
            className={kind === 'menu' ? styles.menu : styles.panel}
          >
            {children(close)}
          </div>
        </>
      ) : null}
    </div>
  );
}
