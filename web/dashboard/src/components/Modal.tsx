// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useCallback, type ReactNode } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  size?: 'default' | 'wide' | 'yaml';
}

export default function Modal({ isOpen, onClose, title, children, size = 'default' }: ModalProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    },
    [onClose],
  );

  useEffect(() => {
    if (isOpen) {
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [isOpen, handleKeyDown]);

  if (!isOpen) return null;

  const isYaml = size === 'yaml';
  const isWide = size === 'wide' || isYaml;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 animate-fade-in">
      <div className="absolute inset-0 bg-slate-950/75 backdrop-blur-md" onClick={onClose} />

      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`surface-panel relative flex w-full animate-scale-in flex-col overflow-hidden rounded-[28px] shadow-2xl ${
          isYaml ? 'max-h-[92vh] min-h-[min(720px,92vh)]' : 'max-h-[85vh]'
        } ${isWide ? 'max-w-[min(96rem,calc(100vw-2rem))]' : 'max-w-2xl'}`}
      >
        <div className="flex shrink-0 items-center justify-between border-b border-slate-800/80 px-6 py-4">
          <div>
            <div className="mb-1 h-1 w-10 rounded-full bg-gradient-to-r from-aether to-cyan-300" />
            <h2 className="text-lg font-semibold text-white">{title}</h2>
          </div>
          <button
            onClick={onClose}
            className="rounded-xl border border-slate-700/80 bg-slate-950/40 p-2 text-slate-400 transition-colors hover:bg-slate-800/80 hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40"
          >
            <X className="w-5 h-5" />
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
