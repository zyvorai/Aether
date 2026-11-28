// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { GO_SHORTCUTS } from '../utils/goShortcuts';
import { modKeyLabel } from '../utils/platformKeys';

export interface HelpShortcut {
  keys: string[];
  description: string;
}

export const helpShortcutGroups: { title: string; shortcuts: HelpShortcut[] }[] = [
  {
    title: 'Everywhere',
    shortcuts: [
      { keys: [modKeyLabel, 'K'], description: 'Command palette' },
      { keys: ['/'], description: 'Focus the search or filter on this page' },
      { keys: ['['], description: 'Collapse or expand the sidebar' },
      { keys: ['p'], description: 'Pin or unpin this page' },
      { keys: ['r'], description: 'Refresh data' },
      { keys: ['?'], description: 'Help (shortcuts & about)' },
    ],
  },
  {
    title: 'Tables and details',
    shortcuts: [
      { keys: ['j'], description: 'Next row or record' },
      { keys: ['k'], description: 'Previous row or record' },
      { keys: ['Enter'], description: 'Open the selected row' },
      { keys: ['Esc'], description: 'Close the side panel or dialog' },
    ],
  },
  {
    title: 'Go to',
    shortcuts: GO_SHORTCUTS.map(({ key, label }) => ({ keys: ['g', key], description: `Go to ${label}` })),
  },
];
