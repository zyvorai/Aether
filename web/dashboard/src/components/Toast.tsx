import { useState, useCallback, useRef, useEffect } from 'react';
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

const colorMap: Record<ToastType, string> = {
  success: 'border-emerald-500/30 bg-zinc-900',
  error: 'border-red-500/30 bg-zinc-900',
  info: 'border-blue-500/30 bg-zinc-900',
};

const iconColorMap: Record<ToastType, string> = {
  success: 'text-emerald-400',
  error: 'text-red-400',
  info: 'text-blue-400',
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
  const Icon = iconMap[item.type];
  const timerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Start the timer bar animation
    if (timerRef.current) {
      timerRef.current.style.transition = 'width 3s linear';
      timerRef.current.style.width = '0%';
    }
  }, []);

  return (
    <div
      className={`pointer-events-auto flex items-start gap-3 min-w-[280px] max-w-sm border rounded-lg px-4 py-3 shadow-lg ${
        colorMap[item.type]
      } ${item.exiting ? 'toast-exit' : 'toast-enter'}`}
    >
      <Icon className={`w-5 h-5 mt-0.5 shrink-0 ${iconColorMap[item.type]}`} />
      <p className="text-sm text-zinc-100 flex-1">{item.message}</p>
      <button
        onClick={() => onDismiss(item.id)}
        className="shrink-0 p-0.5 rounded text-zinc-500 hover:text-zinc-300 transition-colors"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}
