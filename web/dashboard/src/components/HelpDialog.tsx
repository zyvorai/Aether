import { X, Keyboard, Info } from 'lucide-react';
import { helpShortcuts } from './helpShortcuts';
import ZyvorAbout from './ZyvorAbout';
import { useTheme } from '../contexts/ThemeContext';

export type HelpTab = 'shortcuts' | 'about';

interface HelpDialogProps {
  open: boolean;
  tab: HelpTab;
  onClose: () => void;
  onTabChange: (tab: HelpTab) => void;
}

const TABS: { id: HelpTab; label: string; icon: React.ReactNode }[] = [
  { id: 'shortcuts', label: 'Shortcuts', icon: <Keyboard className="w-4 h-4" aria-hidden /> },
  { id: 'about', label: 'About', icon: <Info className="w-4 h-4" aria-hidden /> },
];

function Kbd({ children, light }: { children: string; light?: boolean }) {
  return (
    <kbd className={`px-1.5 py-0.5 rounded text-xs font-mono min-w-[1.5rem] text-center shadow-sm ${
      light
        ? 'bg-slate-100 border border-slate-300 text-slate-700'
        : 'bg-slate-800 border border-slate-600/80 text-slate-300'
    }`}>
      {children}
    </kbd>
  );
}

export default function HelpDialog({ open, tab, onClose, onTabChange }: HelpDialogProps) {
  const { theme } = useTheme();
  const light = theme === 'light';

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[60] bg-black/60 backdrop-blur-sm animate-fade-in flex items-start justify-center pt-[8vh] px-4"
      role="dialog"
      aria-modal="true"
      aria-label="Help"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === 'Escape') onClose();
      }}
    >
      <div
        className={`surface-panel rounded-2xl shadow-2xl w-full max-w-lg border overflow-hidden ${
          light ? 'border-slate-200 bg-white' : 'border-slate-700/40'
        }`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className={`flex items-center justify-between px-5 py-4 border-b ${light ? 'border-slate-200' : 'border-slate-700/40'}`}>
          <h2 className={`text-lg font-semibold ${light ? 'text-slate-900' : 'text-slate-100'}`}>Help</h2>
          <button
            type="button"
            onClick={onClose}
            className={`p-1.5 rounded-lg transition ${
            light
              ? 'text-slate-500 hover:bg-slate-100 hover:text-slate-900'
              : 'text-slate-400 hover:bg-slate-800/80 hover:text-slate-100'
          }`}
            aria-label="Close help"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className={`flex border-b px-2 pt-1 ${light ? 'border-slate-200' : 'border-slate-700/40'}`} role="tablist" aria-label="Help sections">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={tab === t.id}
              onClick={() => onTabChange(t.id)}
              className={`flex items-center gap-2 px-3 py-2.5 text-sm font-medium border-b-2 -mb-px transition-colors ${
                tab === t.id
                  ? 'border-aether text-aether'
                  : `border-transparent ${light ? 'text-slate-500 hover:text-slate-800' : 'text-slate-500 hover:text-slate-300'}`
              }`}
            >
              {t.icon}
              {t.label}
            </button>
          ))}
        </div>

        <div className="max-h-[min(70vh,32rem)] overflow-y-auto p-5">
          {tab === 'shortcuts' ? (
            <div role="tabpanel">
              <ul className="space-y-3 list-none m-0">
                {helpShortcuts.map((s) => (
                  <li key={s.description} className="flex items-center justify-between gap-3">
                    <span className={`text-sm ${light ? 'text-slate-700' : 'text-slate-300'}`}>{s.description}</span>
                    <div className="flex items-center gap-1 shrink-0">
                      {s.keys.map((k, i) => (
                        <span key={`${s.description}-${k}-${i}`} className="flex items-center gap-1">
                          {i > 0 && <span className={`text-xs ${light ? 'text-slate-400' : 'text-slate-600'}`}>+</span>}
                          <Kbd light={light}>{k}</Kbd>
                        </span>
                      ))}
                    </div>
                  </li>
                ))}
              </ul>
              <p className={`text-xs mt-4 pt-3 border-t space-y-1 ${light ? 'text-slate-500 border-slate-200' : 'text-slate-500 border-slate-700/50'}`}>
                <span>
                  Sequence shortcuts use two letters in order (wait under half a second between keys). Shortcuts are
                  disabled while focus is in a field.
                </span>
                <span className="block">
                  Share a workload with URL params, e.g.{' '}
                  <span className={`font-mono ${light ? 'text-slate-600' : 'text-slate-400'}`}>/workloads?workload=my-app&amp;tab=logs</span>.
                </span>
                <span className="block">
                  Open <strong className={light ? 'text-slate-600' : 'text-slate-400'}>Help → About</strong> for product info and documentation
                  links.
                </span>
              </p>
            </div>
          ) : (
            <div role="tabpanel">
              <ZyvorAbout />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
