// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPod|iPad/i.test(navigator.platform);
const modKey = isMac ? '⌘' : 'Ctrl';

export const helpShortcuts: { keys: string[]; description: string }[] = [
  { keys: [modKey, 'K'], description: 'Command palette' },
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
  { keys: ['?'], description: 'Help (shortcuts & about)' },
];
