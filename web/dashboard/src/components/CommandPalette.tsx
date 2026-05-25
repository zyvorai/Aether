import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router';
import type { AppView } from '../types/api';
import { DASHBOARD_VIEWS } from '../utils/dashboardNav';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import { apiPost } from '../utils/api';
import { getRecentViews } from '../utils/recentViews';
import { getRecentActions, pushRecentAction } from '../utils/recentActions';
import { useTheme } from '../contexts/ThemeContext';
import { useAuth } from '../contexts/AuthContext';

type CommandCategory = 'recent' | 'recent-action' | 'navigation' | 'workload' | 'workload-action' | 'action';

interface CommandAction {
  id: string;
  label: string;
  category: CommandCategory;
  searchText: string;
  view?: AppView;
  workloadName?: string;
  workloadTab?: string;
  run?: () => void | Promise<void>;
}

import type { HelpTab } from './HelpDialog';

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onNavigate: (view: AppView) => void;
  workloads: string[];
  onSelectWorkload?: (name: string) => void;
  onRefresh?: () => void;
  onLogout?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
}

const NAV_ITEMS: CommandAction[] = DASHBOARD_VIEWS.map((v) => ({
  id: `nav-${v.view}`,
  label: v.view === 'overview' ? 'Dashboard' : `Go to ${v.label}`,
  category: 'navigation' as const,
  searchText: `${v.label} ${v.subtitle} ${v.view}`,
  view: v.view,
}));

function fuzzyScore(query: string, text: string): number {
  const q = query.toLowerCase().trim();
  const t = text.toLowerCase();
  if (!q) return 1;
  if (t.includes(q)) return 100 + (t.startsWith(q) ? 20 : 0) + (t === q ? 30 : 0);
  let qi = 0;
  let score = 0;
  for (let i = 0; i < t.length && qi < q.length; i++) {
    if (t[i] === q[qi]) {
      score += 10 - Math.min(i, 5);
      qi++;
    }
  }
  return qi === q.length ? score : 0;
}

function paletteToast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPod|iPad/i.test(navigator.platform);

export default function CommandPalette({
  open,
  onClose,
  onNavigate,
  workloads,
  onSelectWorkload,
  onRefresh,
  onLogout,
  onOpenHelp,
}: CommandPaletteProps) {
  const navigate = useNavigate();
  const { theme } = useTheme();
  const light = theme === 'light';
  const { canMutate } = useAuth();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [recentViews, setRecentViews] = useState<AppView[]>([]);
  const [recentActionIds, setRecentActionIds] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const allCommands = useMemo((): CommandAction[] => {
    const workloadItems: CommandAction[] = workloads.flatMap((name) => [
      {
        id: `workload-${name}`,
        label: `Open workload: ${name}`,
        category: 'workload' as const,
        searchText: `workload ${name}`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-logs`,
        label: `View logs: ${name}`,
        category: 'workload-action' as const,
        searchText: `logs ${name} workload`,
        workloadName: name,
        workloadTab: 'logs',
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'logs' }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-scoring`,
        label: `Open scoring: ${name}`,
        category: 'workload-action' as const,
        searchText: `scoring ai intent ${name}`,
        workloadName: name,
        workloadTab: 'scoring',
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'scoring' }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-start`,
        label: `Start workload: ${name}`,
        category: 'workload-action' as const,
        searchText: `start run ${name}`,
        workloadName: name,
        run: async () => {
          if (!canMutate) return;
          const res = await apiPost(`/workloads/${name}/start`);
          paletteToast(
            res.success ? `Started "${name}"` : `Start failed: ${res.error ?? 'unknown error'}`,
            res.success ? 'success' : 'error',
          );
          onRefresh?.();
        },
      },
      {
        id: `workload-${name}-stop`,
        label: `Stop workload: ${name}`,
        category: 'workload-action' as const,
        searchText: `stop halt ${name}`,
        workloadName: name,
        run: async () => {
          if (!canMutate) return;
          const res = await apiPost(`/workloads/${name}/stop`);
          paletteToast(
            res.success ? `Stopped "${name}"` : `Stop failed: ${res.error ?? 'unknown error'}`,
            res.success ? 'success' : 'error',
          );
          onRefresh?.();
        },
      },
    ]);

    const actionItems: CommandAction[] = [];

    if (canMutate) {
      actionItems.unshift({
        id: 'action-deploy',
        label: 'Deploy workload',
        category: 'action',
        searchText: 'deploy yaml create workload',
        run: () => navigate(pathWithQuery(viewToPath('workloads'), { deploy: '1' })),
      });
    }

    actionItems.push(
      {
        id: 'action-refresh',
        label: 'Refresh dashboard',
        category: 'action',
        searchText: 'refresh reload sync',
        run: () => onRefresh?.(),
      },
      {
        id: 'action-editor',
        label: 'Open visual editor',
        category: 'action',
        searchText: 'editor visual form designer',
        view: 'editor',
      },
      {
        id: 'action-compose',
        label: 'Import Docker Compose',
        category: 'action',
        searchText: 'compose docker import',
        view: 'compose',
      },
    );

    if (onLogout) {
      actionItems.push({
        id: 'action-logout',
        label: 'Sign out',
        category: 'action',
        searchText: 'logout sign out exit',
        run: () => onLogout(),
      });
    }

    if (onOpenHelp) {
      actionItems.push(
        {
          id: 'action-help-shortcuts',
          label: 'Help: keyboard shortcuts',
          category: 'action',
          searchText: 'help shortcuts keyboard ?',
          run: () => onOpenHelp('shortcuts'),
        },
        {
          id: 'action-help-about',
          label: 'Help: about Aether',
          category: 'action',
          searchText: 'help about aether zyvor copyright documentation',
          run: () => onOpenHelp('about'),
        },
      );
    }

    return [...NAV_ITEMS, ...workloadItems, ...actionItems];
  }, [workloads, navigate, onSelectWorkload, onRefresh, onLogout, onOpenHelp, canMutate]);

  const recentCommands = useMemo((): CommandAction[] => {
    const items: CommandAction[] = [];
    for (const id of recentActionIds) {
      const cmd = allCommands.find((c) => c.id === id);
      if (cmd) {
        items.push({ ...cmd, category: 'recent-action' });
      }
    }
    for (const view of recentViews) {
      const meta = DASHBOARD_VIEWS.find((v) => v.view === view);
      if (!meta) continue;
      items.push({
        id: `recent-${view}`,
        label: meta.label,
        category: 'recent',
        searchText: `recent ${meta.label} ${meta.subtitle} ${view}`,
        view,
      });
    }
    return items;
  }, [recentViews, recentActionIds, allCommands]);

  const filtered = useMemo(() => {
    const q = query.trim();
    const recentIds = new Set(recentCommands.map((c) => c.id));
    const base = allCommands.filter((cmd) => !recentIds.has(cmd.id));
    const pool = q ? [...recentCommands, ...base] : [...recentCommands, ...base];
    const scored = pool
      .map((cmd) => ({ cmd, score: fuzzyScore(q, cmd.searchText || cmd.label) }))
      .filter(({ score }) => score > 0)
      .sort((a, b) => b.score - a.score);
    return (q ? scored.map(({ cmd }) => cmd) : pool).slice(0, 40);
  }, [allCommands, query, recentCommands]);

  useEffect(() => {
    if (open) {
      setQuery('');
      setSelectedIndex(0);
      setRecentViews(getRecentViews());
      setRecentActionIds(getRecentActions().map((a) => a.id));
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector('[data-selected="true"]');
    selected?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  const executeCommand = useCallback(
    async (cmd: CommandAction) => {
      pushRecentAction({ id: cmd.id, label: cmd.label, searchText: cmd.searchText || cmd.label });
      if (cmd.view) {
        onNavigate(cmd.view);
      } else if (cmd.run) {
        await cmd.run();
      }
      onClose();
    },
    [onNavigate, onClose],
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      void executeCommand(filtered[selectedIndex]);
    } else if (e.key === 'Escape') {
      onClose();
    }
  };

  if (!open) return null;

  const categoryLabels: Record<CommandCategory, string> = {
    recent: 'Recent pages',
    'recent-action': 'Recent actions',
    navigation: 'Navigation',
    workload: 'Workloads',
    'workload-action': 'Workload actions',
    action: 'Actions',
  };

  let lastCategory: CommandCategory | '' = '';

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] px-4"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
    >
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm" />
      <div
        className="relative w-full max-w-xl rounded-[24px] surface-panel overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className={`flex items-center px-4 py-4 border-b ${light ? 'border-slate-200' : 'border-slate-800'}`}>
          <span className="text-slate-500 mr-2 text-sm font-mono">{'>'}</span>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search pages, workloads, and actions…"
            className={`flex-1 bg-transparent text-sm outline-none ${light ? 'text-slate-900 placeholder-slate-400' : 'text-white placeholder-slate-500'}`}
            autoComplete="off"
          />
          <kbd className={`text-xs px-1.5 py-0.5 rounded border ${light ? 'text-slate-500 bg-slate-100 border-slate-300' : 'text-slate-500 bg-slate-800/90 border-slate-700'}`}>ESC</kbd>
        </div>

        <div ref={listRef} className="max-h-[min(24rem,50vh)] overflow-y-auto py-1">
          {filtered.length === 0 ? (
            <div className="px-4 py-8 text-center text-slate-500 text-sm">No results found</div>
          ) : (
            filtered.map((cmd, i) => {
              const showCategory = cmd.category !== lastCategory;
              lastCategory = cmd.category;
              return (
                <div key={cmd.id}>
                  {showCategory ? (
                    <div className="px-4 pt-2 pb-1 text-xs font-medium text-slate-500 uppercase tracking-wider">
                      {categoryLabels[cmd.category]}
                    </div>
                  ) : null}
                  <button
                    type="button"
                    onClick={() => void executeCommand(cmd)}
                    onMouseEnter={() => setSelectedIndex(i)}
                    data-selected={i === selectedIndex}
                    className={`w-full px-4 py-2 flex items-center gap-3 text-sm text-left transition-colors ${
                      i === selectedIndex
                        ? 'bg-aether/20 text-aether'
                        : light
                          ? 'text-slate-700 hover:bg-slate-100'
                          : 'text-slate-300 hover:bg-slate-800/80'
                    }`}
                  >
                    <span className="flex-1 truncate">{cmd.label}</span>
                    {cmd.view ? <span className="text-xs text-slate-600 shrink-0">Navigate</span> : null}
                    {cmd.workloadTab ? <span className="text-xs text-slate-600 shrink-0">{cmd.workloadTab}</span> : null}
                  </button>
                </div>
              );
            })
          )}
        </div>

        <div className={`px-4 py-3 border-t flex flex-wrap items-center gap-x-4 gap-y-1 text-xs ${light ? 'border-slate-200 text-slate-500' : 'border-slate-800 text-slate-500'}`}>
          <span>{isMac ? '⌘K' : 'Ctrl+K'} open</span>
          <span>↑↓ navigate</span>
          <span>Enter select</span>
          <span>Esc close</span>
        </div>
      </div>
    </div>
  );
}
