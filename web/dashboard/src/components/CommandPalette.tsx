import { useState, useEffect, useRef, useCallback } from 'react';
import type { AppView } from '../types/api';
import { DASHBOARD_VIEWS } from '../utils/dashboardNav';

interface CommandAction {
  id: string;
  label: string;
  category: 'navigation' | 'workload' | 'action';
  icon: string;
  view?: AppView;
  workloadName?: string;
}

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onNavigate: (view: AppView) => void;
  workloads: string[];
  onSelectWorkload?: (name: string) => void;
  onRefresh?: () => void;
}

const NAV_ITEMS: CommandAction[] = DASHBOARD_VIEWS.filter((v) => v.view !== 'overview').map((v) => ({
  id: `nav-${v.view}`,
  label: `Go to ${v.label}`,
  category: 'navigation' as const,
  icon: v.label,
  view: v.view,
}));

const ACTION_ITEMS: CommandAction[] = [
  { id: 'action-refresh', label: 'Refresh Dashboard', category: 'action', icon: 'Refresh' },
];

export default function CommandPalette({ open, onClose, onNavigate, workloads, onSelectWorkload, onRefresh }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Build command list
  const allCommands: CommandAction[] = [
    ...NAV_ITEMS,
    ...workloads.map(name => ({
      id: `workload-${name}`,
      label: `Workload: ${name}`,
      category: 'workload' as const,
      icon: 'Workload',
      workloadName: name,
    })),
    ...ACTION_ITEMS,
  ];

  const filtered = query
    ? allCommands.filter(c => c.label.toLowerCase().includes(query.toLowerCase()))
    : allCommands.slice(0, 15);

  useEffect(() => {
    if (open) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  // Scroll the selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector('[data-selected="true"]');
    if (selected) {
      selected.scrollIntoView({ block: 'nearest' });
    }
  }, [selectedIndex]);

  const executeCommand = useCallback((cmd: CommandAction) => {
    if (cmd.view) {
      onNavigate(cmd.view);
    } else if (cmd.workloadName && onSelectWorkload) {
      onNavigate('workloads');
      setTimeout(() => onSelectWorkload(cmd.workloadName!), 100);
    } else if (cmd.id === 'action-refresh' && onRefresh) {
      onRefresh();
    }
    onClose();
  }, [onNavigate, onSelectWorkload, onRefresh, onClose]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex(i => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex(i => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      executeCommand(filtered[selectedIndex]);
    } else if (e.key === 'Escape') {
      onClose();
    }
  };

  if (!open) return null;

  const categoryLabels: Record<string, string> = {
    navigation: 'Navigation',
    workload: 'Workloads',
    action: 'Actions',
  };

  // Group by category
  let lastCategory = '';

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
    >
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm" />
      <div
        className="relative w-full max-w-xl rounded-[24px] surface-panel overflow-hidden"
        onClick={e => e.stopPropagation()}
      >
        {/* Search Input */}
        <div className="flex items-center px-4 py-4 border-b border-slate-800">
          <span className="text-slate-500 mr-2 text-sm font-mono">{'>'}</span>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={e => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a command or search..."
            className="flex-1 bg-transparent text-white text-sm outline-none placeholder-slate-500"
            autoComplete="off"
          />
          <kbd className="text-xs text-slate-500 bg-slate-800/90 px-1.5 py-0.5 rounded border border-slate-700">ESC</kbd>
        </div>

        {/* Results */}
        <div ref={listRef} className="max-h-80 overflow-y-auto py-1">
          {filtered.length === 0 ? (
            <div className="px-4 py-8 text-center text-slate-500 text-sm">No results found</div>
          ) : (
            filtered.map((cmd, i) => {
              const showCategory = cmd.category !== lastCategory;
              lastCategory = cmd.category;
              return (
                <div key={cmd.id}>
                  {showCategory && (
                    <div className="px-4 pt-2 pb-1 text-xs font-medium text-slate-500 uppercase tracking-wider">
                      {categoryLabels[cmd.category] || cmd.category}
                    </div>
                  )}
                  <button
                    onClick={() => executeCommand(cmd)}
                    onMouseEnter={() => setSelectedIndex(i)}
                    data-selected={i === selectedIndex}
                    className={`w-full px-4 py-2 flex items-center gap-3 text-sm text-left transition-colors ${
                      i === selectedIndex ? 'bg-aether/20 text-aether' : 'text-slate-300 hover:bg-slate-800/80'
                    }`}
                  >
                    <span className="text-slate-500 text-xs font-mono w-16 shrink-0">{cmd.icon}</span>
                    <span className="flex-1">{cmd.label}</span>
                    {cmd.view && <span className="text-xs text-slate-600">Navigate</span>}
                  </button>
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="px-4 py-3 border-t border-slate-800 flex items-center gap-4 text-xs text-slate-500">
          <span>Arrow keys navigate</span>
          <span>Enter to select</span>
          <span>Esc to close</span>
        </div>
      </div>
    </div>
  );
}
