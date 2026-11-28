// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

export const isMacPlatform =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPod|iPad/i.test(navigator.platform);

export const modKeyLabel = isMacPlatform ? '⌘' : 'Ctrl';
