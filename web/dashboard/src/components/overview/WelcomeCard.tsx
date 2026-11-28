// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { Keyboard, Search, Star, X } from 'lucide-react';
import { usePreferences } from '../../hooks/usePreferences';
import { modKeyLabel } from '../../utils/platformKeys';

/** Shown until closed: the three things that make a large console quick to use. */
export default function WelcomeCard() {
  const { welcomeDismissed, setPreferences } = usePreferences();
  if (welcomeDismissed) return null;

  const tips = [
    {
      icon: Search,
      title: 'Search anything',
      body: `Press ${modKeyLabel} K to jump to a page, a workload or an action.`,
      action: { label: 'Open search', run: () => window.dispatchEvent(new CustomEvent('aether-open-command-palette')) },
    },
    { icon: Star, title: 'Pin what you use', body: 'Star a page in the sidebar, or press p, to keep it at the top.' },
    { icon: Keyboard, title: 'Stay on the keyboard', body: 'j and k move through tables, / finds a page filter, ? lists every shortcut.' },
  ];

  return (
    <section className="glass welcome-card" aria-label="Getting around" data-testid="overview-welcome">
      <ul className="grid flex-1 gap-4 sm:grid-cols-3">
        {tips.map(({ icon: Icon, title, body, action }) => (
          <li key={title} className="flex items-start gap-2.5">
            <Icon className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden />
            <div className="min-w-0">
              <p className="text-sm font-medium text-foreground">{title}</p>
              <p className="text-xs leading-relaxed text-muted">{body}</p>
              {action ? (
                <button type="button" className="mt-1 text-xs text-primary hover:underline" onClick={action.run}>
                  {action.label}
                </button>
              ) : null}
            </div>
          </li>
        ))}
      </ul>
      <button
        type="button"
        onClick={() => setPreferences({ welcomeDismissed: true })}
        aria-label="Close getting around tips"
        title="Close (Settings can show it again)"
        className="self-start rounded-md p-1 text-subtle hover:bg-hover hover:text-foreground"
      >
        <X className="h-4 w-4" aria-hidden />
      </button>
    </section>
  );
}
