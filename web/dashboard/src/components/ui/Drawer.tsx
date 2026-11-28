// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { ChevronLeft, ChevronRight, X } from 'lucide-react';

interface DrawerProps {
  label: string;
  onClose: () => void;
  /** Step to the previous or next record; omit to hide the stepper. */
  stepper?: { index: number; count: number; onStep: (delta: 1 | -1) => void };
  /** Changing this scrolls the drawer back to the top. */
  contentKey?: string;
  testId?: string;
  /** Pass false when the content already has its own close button. */
  closeButton?: boolean;
  children: ReactNode;
}

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** True while a dialog opened from inside the drawer sits on top of it. */
function coveredBy(panel: HTMLElement): boolean {
  return Array.from(document.querySelectorAll('[aria-modal="true"]')).some(
    (other) => other !== panel && !panel.contains(other),
  );
}

/** Side sheet for one record: Escape closes, focus stays inside and returns to the opener. */
export default function Drawer({ label, onClose, stepper, contentKey, testId, closeButton = true, children }: DrawerProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const latest = useRef({ onClose, stepper });
  latest.current = { onClose, stepper };

  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const overflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    if (panel && !panel.contains(document.activeElement)) panel.focus();

    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (!panel || coveredBy(panel) || event.metaKey || event.ctrlKey || event.altKey) return;
      const target = event.target as HTMLElement | null;
      const typing = Boolean(target && (['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName) || target.isContentEditable));
      if (event.key === 'Escape') {
        if (typing && (target as HTMLInputElement).value) return;
        event.preventDefault();
        latest.current.onClose();
        return;
      }
      if (!typing && (event.key === 'j' || event.key === 'k') && latest.current.stepper) {
        event.preventDefault();
        latest.current.stepper.onStep(event.key === 'j' ? 1 : -1);
        return;
      }
      if (event.key !== 'Tab') return;
      const items = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.offsetParent !== null);
      const first = items[0];
      const last = items[items.length - 1];
      if (!first || !last) return;
      const inside = panel.contains(document.activeElement) && document.activeElement !== panel;
      if (event.shiftKey && (!inside || document.activeElement === first)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && (!inside || document.activeElement === last)) {
        event.preventDefault();
        first.focus();
      }
    };
    // Capture phase: decide whether a dialog is on top before that dialog closes itself.
    document.addEventListener('keydown', onKeyDown, true);
    return () => {
      document.removeEventListener('keydown', onKeyDown, true);
      document.body.style.overflow = overflow;
      if (opener?.isConnected) opener.focus();
    };
  }, []);

  useEffect(() => {
    bodyRef.current?.scrollTo({ top: 0 });
    // Stepping replaces the content; keep focus in the panel if its element went away.
    const panel = panelRef.current;
    if (panel && !panel.contains(document.activeElement)) panel.focus();
  }, [contentKey]);

  return createPortal(
    <div className="drawer-root">
      <div className="drawer-scrim" onClick={onClose} aria-hidden />
      <div ref={panelRef} className="drawer-panel" role="dialog" aria-modal="true" aria-label={label} tabIndex={-1} data-testid={testId}>
        <div className="drawer-bar">
          {stepper && stepper.count > 1 ? (
            <div className="flex items-center gap-1">
              <button type="button" className="drawer-step" onClick={() => stepper.onStep(-1)} disabled={stepper.index <= 0} aria-label="Previous record" title="Previous ( k )">
                <ChevronLeft className="h-4 w-4" aria-hidden />
              </button>
              <button type="button" className="drawer-step" onClick={() => stepper.onStep(1)} disabled={stepper.index >= stepper.count - 1} aria-label="Next record" title="Next ( j )">
                <ChevronRight className="h-4 w-4" aria-hidden />
              </button>
              <span className="ml-1 text-xs tabular-nums text-subtle" aria-live="polite">
                {stepper.index + 1} of {stepper.count}
              </span>
            </div>
          ) : (
            <span />
          )}
          {closeButton ? (
            <button type="button" onClick={onClose} className="drawer-close">
              <X className="h-3.5 w-3.5" aria-hidden />
              Close <kbd>Esc</kbd>
            </button>
          ) : (
            <span className="drawer-hint">
              <kbd>Esc</kbd> closes
            </span>
          )}
        </div>
        <div ref={bodyRef} className="drawer-body">
          {children}
        </div>
      </div>
    </div>,
    document.body,
  );
}
