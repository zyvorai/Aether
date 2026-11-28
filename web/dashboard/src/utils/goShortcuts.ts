// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { AppView } from '../types/api';

/** `g` followed by a letter. One table drives both the handler and the help dialog. */
export const GO_SHORTCUTS: { key: string; view: AppView; label: string }[] = [
  { key: 'd', view: 'overview', label: 'Overview' },
  { key: 'w', view: 'workloads', label: 'Workloads' },
  { key: 'a', view: 'applications', label: 'Applications' },
  { key: 'c', view: 'clusters', label: 'Cluster Browser' },
  { key: 'h', view: 'health', label: 'Health Monitor' },
  { key: 'e', view: 'events', label: 'Events' },
  { key: 'm', view: 'metrics', label: 'Metrics' },
  { key: 'b', view: 'backups', label: 'Backups' },
  { key: 's', view: 'secrets', label: 'Secrets' },
  { key: 'u', view: 'audit', label: 'Audit Trail' },
  { key: 'y', view: 'policy', label: 'Policy Check' },
  { key: 't', view: 'settings', label: 'Settings' },
];
