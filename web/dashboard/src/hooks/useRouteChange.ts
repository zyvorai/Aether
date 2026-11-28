// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useRef, useState } from 'react';
import type { AppView } from '../types/api';
import { getViewMeta } from '../utils/dashboardNav';
import { pageTitle } from '../utils/pageTitle';

/**
 * What a full page load would do for a single-page app: set the document title,
 * start at the top, move focus to the content and announce the new page.
 */
export function useRouteChange(view: AppView): string {
  const previous = useRef(view);
  const [announcement, setAnnouncement] = useState('');

  useEffect(() => {
    document.title = pageTitle(view);
    if (previous.current === view) return;
    previous.current = view;
    window.scrollTo({ top: 0 });
    setAnnouncement(`${getViewMeta(view).label} page`);
    const main = document.getElementById('main-content');
    // A page that focuses its own field, or an open dialog, keeps the focus.
    if (main && !main.contains(document.activeElement) && !document.querySelector('[aria-modal="true"]')) {
      main.focus({ preventScroll: true });
    }
  }, [view]);

  return announcement;
}
