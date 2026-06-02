// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useCallback, useRef, type ReactNode } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  size?: 'default' | 'wide' | 'yaml';
}

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export default function Modal({ isOpen, onClose, title, children, size = 'default' }: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<Element | null>(null);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
        return;
      }
      if (e.key !== 'Tab' || !panelRef.current) return;

      const focusable = Array.from(panelRef.current.querySelectorAll(FOCUSABLE)) as HTMLElement[];
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const active = document.activeElement as HTMLElement | null;

      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    },
    [onClose],
  );

  useEffect(() => {
    if (isOpen) {
      triggerRef.current = document.activeElement;
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
      requestAnimationFrame(() => {
        const first = panelRef.current?.querySelector(FOCUSABLE) as HTMLElement | null;
        first?.focus();
      });
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [isOpen, handleKeyDown]);

  useEffect(() => {
    if (!isOpen && triggerRef.current instanceof HTMLElement) {
      triggerRef.current.focus();
      triggerRef.current = null;
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const isYaml = size === 'yaml';
  const isWide = size === 'wide' || isYaml;

  return (
    <div className="fixed inset-0 z-50 flex animate-fade-in items-center justify-center p-4">
      <div className="glass-modal-backdrop" onClick={onClose} aria-hidden />

      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`glass-modal-panel animate-scale-in shadow-2xl ${
          isYaml ? 'max-h-[92vh] min-h-[min(720px,92vh)]' : 'max-h-[85vh]'
        } ${isWide ? 'max-w-[min(96rem,calc(100vw-2rem))] max-sm:max-w-[calc(100vw-1rem)] max-sm:rounded-none max-sm:max-h-[100dvh]' : 'max-w-2xl max-sm:max-w-[calc(100vw-1rem)]'}`}
      >
        <div className="flex shrink-0 items-center justify-between glass-divider-b px-6 py-4">
          <div>
            <div className="mb-1 h-1 w-10 rounded-full bg-gradient-to-r from-aether to-aether-ai" />
            <h2 className="text-lg font-semibold text-white">{title}</h2>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close dialog"
            className="quick-link-chip p-2 text-slate-400 transition-colors hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <div
          className={`px-6 py-5 ${isYaml ? 'flex min-h-0 flex-1 flex-col overflow-hidden' : 'overflow-y-auto'}`}
        >
          {children}
        </div>
      </div>
    </div>
  );
}
