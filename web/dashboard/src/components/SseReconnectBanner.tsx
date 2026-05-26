import { Radio, RefreshCw } from 'lucide-react';

interface SseReconnectBannerProps {
  onRefresh: () => void;
}

export default function SseReconnectBanner({ onRefresh }: SseReconnectBannerProps) {
  return (
    <div
      role="status"
      className="border-b border-amber-500/30 bg-amber-500/10 px-4 py-2.5 text-sm text-amber-200"
    >
      <div className="dash-content flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <Radio className="h-4 w-4 shrink-0 text-amber-400" aria-hidden />
          <span>
            Live updates disconnected — dashboard is polling every 60 seconds until SSE reconnects.
          </span>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-amber-500/40 px-3 py-1 text-xs font-medium text-amber-100 transition-colors hover:bg-amber-500/15"
        >
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh now
        </button>
      </div>
    </div>
  );
}
