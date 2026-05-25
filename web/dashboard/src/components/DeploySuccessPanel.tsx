import { FileText, ExternalLink } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';

interface DeploySuccessPanelProps {
  name: string;
  status: string;
  onViewLogs: () => void;
  onClose: () => void;
}

export default function DeploySuccessPanel({ name, status, onViewLogs, onClose }: DeploySuccessPanelProps) {
  const { theme } = useTheme();
  const light = theme === 'light';

  return (
    <div
      data-testid="deploy-success-panel"
      className={`rounded-xl border p-4 shrink-0 ${
        light ? 'border-emerald-200 bg-emerald-50' : 'border-emerald-800/60 bg-emerald-950/40'
      }`}
    >
      <p className={`text-sm font-medium ${light ? 'text-emerald-900' : 'text-emerald-300'}`}>
        Deployed &quot;{name}&quot; successfully
      </p>
      <p className={`mt-1 text-xs ${light ? 'text-emerald-800/80' : 'text-emerald-400/90'}`}>
        Rollout status: {status}
      </p>
      <div className="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={onViewLogs}
          className="inline-flex items-center gap-1.5 rounded-lg bg-aether px-3 py-2 text-xs font-medium text-white hover:bg-aether-light"
        >
          <FileText className="w-3.5 h-3.5" />
          View logs
        </button>
        <button
          type="button"
          onClick={onClose}
          className={`inline-flex items-center gap-1.5 rounded-lg border px-3 py-2 text-xs font-medium ${
            light
              ? 'border-slate-300 bg-white text-slate-700 hover:bg-slate-50'
              : 'border-zinc-600 bg-zinc-900 text-zinc-300 hover:bg-zinc-800'
          }`}
        >
          <ExternalLink className="w-3.5 h-3.5" />
          Close
        </button>
      </div>
    </div>
  );
}
