// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useCallback, useRef } from 'react';
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

export function useToast() {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const nextId = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) =>
      prev.map((t) => (t.id === id ? { ...t, exiting: true } : t)),
    );
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 300);
  }, []);

  const toast = useCallback(
    (message: string, type: ToastType = 'info') => {
      const id = nextId.current++;
      setToasts((prev) => [...prev, { id, message, type, exiting: false }]);
      setTimeout(() => dismiss(id), 3000);
    },
    [dismiss],
  );

  function ToastContainer() {
    return (
      <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none" role="status" aria-live="polite" aria-atomic="true">
        {toasts.map((t) => (
          <ToastCard key={t.id} item={t} onDismiss={dismiss} />
        ))}
      </div>
    );
  }

  return { toast, ToastContainer };
}

function ToastCard({ item, onDismiss }: { item: ToastItem; onDismiss: (id: number) => void }) {
  const Icon = iconMap[item.type];

  const borderBg =
    item.type === 'success'
      ? 'border-success/30 glass-drawer'
      : item.type === 'error'
        ? 'border-danger/30 glass-drawer'
        : 'border-primary/30 glass-drawer';

  const iconColor =
    item.type === 'success' ? 'text-success' : item.type === 'error' ? 'text-danger' : 'text-primary';

  return (
    <div
      className={`pointer-events-auto flex min-w-[280px] max-w-sm items-start gap-3 rounded-2xl border px-4 py-3 shadow-[0_18px_48px_rgba(2,6,23,0.45)] backdrop-blur-xl ${borderBg} ${
        item.exiting ? 'toast-exit' : 'toast-enter'
      }`}
    >
      <Icon className={`mt-0.5 h-5 w-5 shrink-0 ${iconColor}`} />
      <p className="flex-1 text-sm text-foreground">{item.message}</p>
      <button
        type="button"
        onClick={() => onDismiss(item.id)}
        aria-label="Dismiss notification"
        className="shrink-0 rounded p-0.5 text-subtle transition-colors hover:text-muted"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
