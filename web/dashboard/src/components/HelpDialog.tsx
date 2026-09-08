// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { X, Keyboard, Info } from 'lucide-react';
import { helpShortcuts } from './helpShortcuts';
import ZyvorAbout from './ZyvorAbout';

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

function Kbd({ children }: { children: string }) {
  return (
    <kbd className="min-w-[1.5rem] rounded border glass-divider/80 glass-inset-surface px-1.5 py-0.5 text-center text-xs font-mono text-muted shadow-sm">
      {children}
    </kbd>
  );
}

export default function HelpDialog({ open, tab, onClose, onTabChange }: HelpDialogProps) {
  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[60] flex items-start justify-center px-4 pt-[8vh] animate-fade-in"
      role="dialog"
      aria-modal="true"
      aria-label="Help"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === 'Escape') onClose();
      }}
    >
      <div className="glass-modal-backdrop fixed inset-0" aria-hidden />
      <div
        className="glass rounded-[var(--radius-xl)] relative w-full max-w-lg overflow-hidden shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between glass-divider-b/40 px-5 py-4">
          <h2 className="text-lg font-semibold text-foreground">Help</h2>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg p-1.5 text-muted transition glass-inset-hover hover:text-foreground"
            aria-label="Close help"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex flex-wrap gap-2 glass-divider-b px-4 py-3" role="tablist" aria-label="Help sections">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={tab === t.id}
              onClick={() => onTabChange(t.id)}
              className={`glass-tab tab-chip flex items-center gap-2 ${tab === t.id ? 'glass-tab-active tab-chip-active glass-tab-active' : ''}`}
            >
              {t.icon}
              {t.label}
            </button>
          ))}
        </div>

        <div className="max-h-[min(70vh,32rem)] overflow-y-auto p-5">
          {tab === 'shortcuts' ? (
            <div role="tabpanel">
              <ul className="m-0 list-none space-y-3">
                {helpShortcuts.map((s) => (
                  <li key={s.description} className="flex items-center justify-between gap-3">
                    <span className="text-sm text-muted">{s.description}</span>
                    <div className="flex shrink-0 items-center gap-1">
                      {s.keys.map((k, i) => (
                        <span key={`${s.description}-${k}-${i}`} className="flex items-center gap-1">
                          {i > 0 && <span className="text-xs text-subtle">+</span>}
                          <Kbd>{k}</Kbd>
                        </span>
                      ))}
                    </div>
                  </li>
                ))}
              </ul>
              <p className="mt-4 space-y-1 glass-divider-t pt-3 text-xs text-subtle">
                <span>
                  Sequence shortcuts use two letters in order (wait under half a second between keys). Shortcuts are
                  disabled while focus is in a field.
                </span>
                <span className="block">
                  Share a workload with URL params, e.g.{' '}
                  <span className="font-mono text-muted">/workloads?workload=my-app&amp;tab=logs</span>.
                </span>
                <span className="block">
                  Open <strong className="text-muted">Help → About</strong> for product info and documentation
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
