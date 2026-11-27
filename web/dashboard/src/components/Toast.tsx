// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { CheckCircle2, XCircle, Info, X } from 'lucide-react';

type ToastType = 'success' | 'error' | 'info';

interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
  exiting: boolean;
}

const iconMap: Record<ToastType, typeof CheckCircle2> = {
  success: CheckCircle2,
  error: XCircle,
  info: Info,
};

/** Errors stay long enough to read and copy. */
const LIFETIME_MS: Record<ToastType, number> = { success: 4000, info: 4000, error: 12000 };
const MAX_VISIBLE = 4;

let toasts: ToastItem[] = [];
let nextId = 0;
const listeners = new Set<() => void>();

function publish(next: ToastItem[]) {
  toasts = next;
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function dismiss(id: number) {
  publish(toasts.map((t) => (t.id === id ? { ...t, exiting: true } : t)));
  setTimeout(() => publish(toasts.filter((t) => t.id !== id)), 300);
}

// Module-level so its identity never changes: a component created inside the
// hook was remounted (and re-animated) on every render of the app.
function ToastContainer() {
  const items = useSyncExternalStore(subscribe, () => toasts, () => toasts);
  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-[60] flex max-w-[calc(100vw-2rem)] flex-col gap-2" role="status" aria-live="polite">
      {items.map((t) => (
        <ToastCard key={t.id} item={t} />
      ))}
    </div>
  );
}

export function useToast() {
  const toast = useCallback((message: string, type: ToastType = 'info') => {
    const same = toasts.find((t) => t.message === message && t.type === type && !t.exiting);
    if (same) return;
    const item = { id: nextId++, message, type, exiting: false };
    publish([...toasts.slice(-(MAX_VISIBLE - 1)), item]);
  }, []);

  return { toast, ToastContainer };
}

function ToastCard({ item }: { item: ToastItem }) {
  const Icon = iconMap[item.type];
  const [held, setHeld] = useState(false);
  const remaining = useRef(LIFETIME_MS[item.type]);

  // The countdown pauses while the pointer or focus is on the message.
  useEffect(() => {
    if (held || item.exiting) return;
    const started = Date.now();
    const timer = window.setTimeout(() => dismiss(item.id), remaining.current);
    return () => {
      window.clearTimeout(timer);
      remaining.current = Math.max(1000, remaining.current - (Date.now() - started));
    };
  }, [held, item.exiting, item.id]);

  const border = item.type === 'success' ? 'border-success/30' : item.type === 'error' ? 'border-danger/30' : 'border-primary/30';
  const iconColor = item.type === 'success' ? 'text-success' : item.type === 'error' ? 'text-danger' : 'text-primary';

  return (
    <div
      className={`glass-drawer pointer-events-auto flex min-w-[min(280px,100%)] max-w-sm items-start gap-3 rounded-2xl border px-4 py-3 ${border} ${
        item.exiting ? 'toast-exit' : 'toast-enter'
      }`}
      role={item.type === 'error' ? 'alert' : undefined}
      onMouseEnter={() => setHeld(true)}
      onMouseLeave={() => setHeld(false)}
      onFocus={() => setHeld(true)}
      onBlur={() => setHeld(false)}
    >
      <Icon className={`mt-0.5 h-5 w-5 shrink-0 ${iconColor}`} aria-hidden />
      <p className="flex-1 break-words text-sm text-foreground">{item.message}</p>
      <button
        type="button"
        onClick={() => dismiss(item.id)}
        aria-label="Dismiss notification"
        className="shrink-0 rounded-sm p-0.5 text-subtle transition-colors hover:text-muted"
      >
        <X className="h-4 w-4" aria-hidden />
      </button>
    </div>
  );
}
