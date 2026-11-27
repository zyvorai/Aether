// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState, type ReactNode } from 'react';
import GlobalNav from './layout/GlobalNav/GlobalNav';
import AetherSidebar from './layout/AetherSidebar/AetherSidebar';
import Breadcrumb from './Breadcrumb';
import { useRouteChange } from '../hooks/useRouteChange';
import { usePreferences } from '../hooks/usePreferences';
import type { LiveStatusProps } from './layout/GlobalNav/LiveStatus';
import ZyraRail from './ZyraRail';
import CriticalIssueNotifier from './CriticalIssueNotifier';
import SseReconnectBanner from './SseReconnectBanner';
import VersionRefreshBanner from './VersionRefreshBanner';
import ViewerBanner from './ViewerBanner';
import type { HelpTab } from './HelpDialog';
import type { AppView } from '../types/api';

interface DashboardShellProps {
  shellClass: string;
  currentView: AppView;
  heroTitle: string;
  heroSubtitle: string;
  username: string;
  lastRefreshed: Date;
  sseConnected: boolean;
  sseBannerVisible?: boolean;
  livePaused: boolean;
  heldUpdates: number;
  onTogglePause: () => void;
  onNavigate: (view: AppView) => void;
  onLogout: () => void;
  onRefresh: () => void;
  onOpenCommandPalette?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
  breadcrumbWorkload?: string;
  children: ReactNode;
  commandPalette: ReactNode;
  helpDialog: ReactNode;
  toastContainer: ReactNode;
  refreshKey?: number;
}

export default function DashboardShell({
  shellClass,
  currentView,
  username,
  lastRefreshed,
  sseConnected,
  sseBannerVisible = false,
  livePaused,
  heldUpdates,
  onTogglePause,
  onNavigate,
  onLogout,
  onRefresh,
  onOpenCommandPalette,
  onOpenHelp,
  breadcrumbWorkload,
  children,
  commandPalette,
  helpDialog,
  toastContainer,
  refreshKey = 0,
}: DashboardShellProps) {
  const isZyraView = currentView === 'zyra' || currentView === 'copilot';
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const { density } = usePreferences();
  const announcement = useRouteChange(currentView);
  const live: LiveStatusProps = {
    connected: sseConnected,
    paused: livePaused,
    heldUpdates,
    lastRefreshed,
    onTogglePause,
    onRefresh,
  };

  useEffect(() => {
    document.documentElement.setAttribute('data-density', density);
  }, [density]);

  useEffect(() => {
    setMobileSidebarOpen(false);
  }, [currentView]);

  useEffect(() => {
    if (!mobileSidebarOpen) return;
    const previous = document.activeElement as HTMLElement | null;
    // Focus the current page rather than the filter, which would raise a phone keyboard.
    const dialog = document.querySelector<HTMLElement>('[role="dialog"][aria-label="Navigation"]');
    (dialog?.querySelector<HTMLElement>('[aria-current="page"]') ?? dialog?.querySelector<HTMLElement>('nav a'))?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setMobileSidebarOpen(false);
    };
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('keydown', onKeyDown);
      previous?.focus();
    };
  }, [mobileSidebarOpen]);

  return (
    <div className={shellClass}>
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-xl focus:border focus:border-border focus:bg-surface-elevated focus:shadow-card focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-primary focus:shadow-lg"
      >
        Skip to content
      </a>
      <GlobalNav
        currentView={currentView}
        onNavigate={onNavigate}
        username={username}
        onLogout={onLogout}
        onSearchClick={onOpenCommandPalette}
        onOpenHelp={onOpenHelp}
        onOpenMobileSidebar={() => setMobileSidebarOpen(true)}
        live={live}
        refreshKey={refreshKey}
      />
      <p className="sr-only" role="status" aria-live="polite">{announcement}</p>
      <div className="relative z-[1]">
        <ViewerBanner />
        {sseBannerVisible ? <SseReconnectBanner onRefresh={onRefresh} /> : null}
        <VersionRefreshBanner />
      </div>
      <div className="relative z-[1] flex min-h-0 flex-1">
        <AetherSidebar currentView={currentView} onNavigate={onNavigate} />
        <main id="main-content" tabIndex={-1} className="min-w-0 flex-1 dash-content py-4 outline-hidden lg:px-8 lg:py-5">
          <Breadcrumb currentView={currentView} onNavigate={onNavigate} workloadName={breadcrumbWorkload} />
          {children}
        </main>
        {isZyraView ? (
          <ZyraRail collapsed={false} onCollapsedChange={() => onNavigate('overview')} />
        ) : null}
      </div>
      {mobileSidebarOpen ? (
        <div className="fixed inset-0 z-50 lg:hidden">
          <button
            type="button"
            className="glass-modal-backdrop absolute inset-0"
            aria-label="Close navigation"
            onClick={() => setMobileSidebarOpen(false)}
          />
          <div className="absolute inset-y-0 left-0 flex animate-fade-in" role="dialog" aria-modal="true" aria-label="Navigation">
            <AetherSidebar
              currentView={currentView}
              onNavigate={onNavigate}
              mobile
              onNavigateMobile={() => setMobileSidebarOpen(false)}
            />
          </div>
        </div>
      ) : null}
      <CriticalIssueNotifier refreshKey={refreshKey} />
      {commandPalette}
      {helpDialog}
      {toastContainer}
    </div>
  );
}
