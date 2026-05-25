import { useState, useCallback, useRef } from 'react';
import { CheckCircle2, XCircle, Info, X } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';

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
      <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
        {toasts.map((t) => (
          <ToastCard key={t.id} item={t} onDismiss={dismiss} />
        ))}
      </div>
    );
  }

  return { toast, ToastContainer };
}

function ToastCard({ item, onDismiss }: { item: ToastItem; onDismiss: (id: number) => void }) {
  const { theme } = useTheme();
  const light = theme === 'light';
  const Icon = iconMap[item.type];

  const borderBg =
    item.type === 'success'
      ? light
        ? 'border-emerald-300 bg-white'
        : 'border-emerald-500/30 bg-slate-900/95'
      : item.type === 'error'
        ? light
          ? 'border-red-300 bg-white'
          : 'border-red-500/30 bg-slate-900/95'
        : light
          ? 'border-orange-300 bg-white'
          : 'border-aether/30 bg-slate-900/95';

  const iconColor =
    item.type === 'success' ? 'text-emerald-500' : item.type === 'error' ? 'text-red-500' : 'text-aether';

  return (
    <div
      className={`pointer-events-auto flex items-start gap-3 min-w-[280px] max-w-sm border rounded-2xl px-4 py-3 shadow-[0_18px_48px_rgba(2,6,23,0.45)] ${borderBg} ${
        item.exiting ? 'toast-exit' : 'toast-enter'
      }`}
    >
      <Icon className={`w-5 h-5 mt-0.5 shrink-0 ${iconColor}`} />
      <p className={`text-sm flex-1 ${light ? 'text-slate-800' : 'text-slate-100'}`}>{item.message}</p>
      <button
        onClick={() => onDismiss(item.id)}
        className={`shrink-0 p-0.5 rounded transition-colors ${light ? 'text-slate-500 hover:text-slate-800' : 'text-slate-500 hover:text-slate-300'}`}
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}
