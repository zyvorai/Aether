import { FileText, ExternalLink } from 'lucide-react';

interface DeploySuccessPanelProps {
  name: string;
  status: string;
  onViewLogs: () => void;
  onClose: () => void;
}

export default function DeploySuccessPanel({ name, status, onViewLogs, onClose }: DeploySuccessPanelProps) {
  return (
    <div
      data-testid="deploy-success-panel"
      className="shrink-0 rounded-xl border border-emerald-800/60 bg-emerald-950/40 p-4"
    >
      <p className="text-sm font-medium text-emerald-300">
        Deployed &quot;{name}&quot; successfully
      </p>
      <p className="mt-1 text-xs text-emerald-400/90">Rollout status: {status}</p>
      <div className="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={onViewLogs}
          className="inline-flex items-center gap-1.5 rounded-lg bg-aether px-3 py-2 text-xs font-medium text-white hover:bg-aether-light"
        >
          <FileText className="h-3.5 w-3.5" />
          View logs
        </button>
        <button
          type="button"
          onClick={onClose}
          className="inline-flex items-center gap-1.5 rounded-lg border border-zinc-600 bg-zinc-900 px-3 py-2 text-xs font-medium text-zinc-300 hover:bg-zinc-800"
        >
          <ExternalLink className="h-3.5 w-3.5" />
          Close
        </button>
      </div>
    </div>
  );
}
