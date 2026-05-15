import { X } from 'lucide-react';

const shortcuts: { keys: string[]; description: string }[] = [
  { keys: ['Ctrl', 'K'], description: 'Command palette' },
  { keys: ['g', 'd'], description: 'Go to Dashboard' },
  { keys: ['g', 'w'], description: 'Go to Workloads' },
  { keys: ['g', 'c'], description: 'Go to Cluster Browser' },
  { keys: ['g', 'h'], description: 'Go to Health Monitor' },
  { keys: ['g', 'e'], description: 'Go to Events' },
  { keys: ['g', 'b'], description: 'Go to Backups' },
  { keys: ['g', 'u'], description: 'Go to Audit Trail' },
  { keys: ['g', 'm'], description: 'Go to Metrics' },
  { keys: ['g', 'y'], description: 'Go to Policy Check' },
  { keys: ['g', 's'], description: 'Go to Secrets' },
  { keys: ['r'], description: 'Refresh data' },
  { keys: ['?'], description: 'Keyboard shortcuts (Shift + /)' },
];

function Kbd({ children }: { children: string }) {
  return (
    <kbd className="px-1.5 py-0.5 bg-slate-800 border border-slate-600/80 rounded text-xs font-mono text-slate-300 min-w-[1.5rem] text-center shadow-sm">
      {children}
    </kbd>
  );
}

interface ShortcutsHelpProps {
  onClose: () => void;
}

export default function ShortcutsHelp({ onClose }: ShortcutsHelpProps) {
  return (
    <div
      className="fixed inset-0 z-[60] bg-black/60 backdrop-blur-sm animate-fade-in flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="aether-shortcuts-title"
      onClick={onClose}
    >
      <div
        className="surface-panel rounded-2xl shadow-2xl w-full max-w-md border border-slate-700/40"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-700/40">
          <h2 id="aether-shortcuts-title" className="text-lg font-semibold text-slate-100">
            Keyboard Shortcuts
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 hover:bg-slate-800/80 rounded-lg transition text-slate-400 hover:text-slate-100"
            aria-label="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
        <ul className="p-5 space-y-3 list-none m-0">
          {shortcuts.map((s) => (
            <li key={s.description} className="flex items-center justify-between gap-3">
              <span className="text-sm text-slate-300">{s.description}</span>
              <div className="flex items-center gap-1 shrink-0">
                {s.keys.map((k, i) => (
                  <span key={`${s.description}-${k}-${i}`} className="flex items-center gap-1">
                    {i > 0 && <span className="text-slate-600 text-xs">+</span>}
                    <Kbd>{k}</Kbd>
                  </span>
                ))}
              </div>
            </li>
          ))}
        </ul>
        <div className="px-5 py-3 border-t border-slate-700/40 text-xs text-slate-500">
          Sequence shortcuts use two letters in order (wait under half a second between keys). Shortcuts are disabled
          while focus is in a field. Each screen has a URL (for example <span className="font-mono text-slate-400">/workloads</span>) you can bookmark or share.
        </div>
      </div>
    </div>
  );
}
