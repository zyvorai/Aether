// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { AppView } from '../types/api';
import type { Tone } from '../components/layout/PageHero';
import { getSectionId, type NavSectionId } from './navSections';

/** One colour per sidebar section, so a page's tile says where it lives. */
const SECTION_TONES: Record<NavSectionId, Tone> = {
  workspace: 'sky',
  build: 'rust',
  observe: 'pink',
  infrastructure: 'teal',
  intelligence: 'violet',
  security: 'emerald',
  platform: 'slate',
};

export function sectionTone(view: AppView): Tone {
  return SECTION_TONES[getSectionId(view)];
}
