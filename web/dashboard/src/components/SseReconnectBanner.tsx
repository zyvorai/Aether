import { Radio, RefreshCw } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';

interface SseReconnectBannerProps {
  onRefresh: () => void;
}

export default function SseReconnectBanner({ onRefresh }: SseReconnectBannerProps) {
  const { theme } = useTheme();
  const light = theme === 'light';

  return (
    <div
      role="status"
      className={`border-b px-4 py-2.5 text-sm ${
        light
          ? 'border-amber-300 bg-amber-50 text-amber-900'
          : 'border-amber-500/30 bg-amber-500/10 text-amber-200'
      }`}
    >
      <div className="dash-content flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <Radio className={`h-4 w-4 shrink-0 ${light ? 'text-amber-600' : 'text-amber-400'}`} aria-hidden />
          <span>
            Live updates disconnected — dashboard is polling every 60 seconds until SSE reconnects.
          </span>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          className={`inline-flex items-center gap-1.5 rounded-lg border px-3 py-1 text-xs font-medium transition-colors shrink-0 ${
            light
              ? 'border-amber-400 text-amber-900 hover:bg-amber-100'
              : 'border-amber-500/40 text-amber-100 hover:bg-amber-500/15'
          }`}
        >
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh now
        </button>
      </div>
    </div>
  );
}
